use macroquad::prelude::*;

use std::clone::Clone;
use std::marker::Copy;

const WINDOW_WIDTH: i32 = 1000;
const WINDOW_HEIGHT: i32 = 600;

const QUADTREE_REGION_LIMIT: usize = 10;
const PARTICLE_SPAWN_INTERVAL: f32 = 0.05;
const PARTICLE_SPAWN_RATE: f32 = 2000.0;
const PARTICLE_RADIUS: f32 = 1.0;

struct QuadNode {
    region: Rect,
    points: Vec<(u32, Vec2)>,
    regions: Vec<Box<QuadNode>>,
}

impl QuadNode {
    fn new(region: Rect) -> Self {
        Self {
            region,
            points: Vec::new(),
            regions: QuadNode::make_regions(&region),
        }
    }

    fn new_empty(region: Rect) -> Self {
        Self {
            region,
            points: Vec::new(),
            regions: Vec::new(),
        }
    }

    fn make_regions(region: &Rect) -> Vec<Box<QuadNode>> {
        let x = region.x;
        let y = region.y;
        let hw = region.w / 2.0;
        let hh = region.h / 2.0;
        vec![
            Box::new(QuadNode::new_empty(Rect::new(x, y, hw, hh))),
            Box::new(QuadNode::new_empty(Rect::new(x + hw, y, hw, hh))),
            Box::new(QuadNode::new_empty(Rect::new(x, y + hh, hw, hh))),
            Box::new(QuadNode::new_empty(Rect::new(x + hw, y + hh, hw, hh))),
        ]
    }

    fn query(&self, query_area: &Rect) -> Vec<(u32, Vec2)> {
        let mut ids = Vec::new();
        for node in &self.regions {
            if node.in_region(query_area) {
                if node.regions.len() > 0 {
                    ids.append(&mut node.query(query_area));
                } else {
                    ids.append(&mut node.points.clone());
                }
            }
        }
        ids
    }

    fn draw(&self) {
        let r = self.region;
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, GREEN);
        for region in &self.regions {
            region.draw();
        }
    }

    fn add(&mut self, id: u32, position: &Vec2) {
        if !self.region.contains(position.clone()) {
            return;
        }

        if self.regions.len() == 0 {
            if self.points.len() == QUADTREE_REGION_LIMIT {
                self.split();
                self.add(id, position);
            } else {
                self.points.push((id, position.clone()));
            }

            return;
        }

        for region in &mut self.regions {
            region.add(id, position);
        }
    }

    fn split(&mut self) {
        self.regions = QuadNode::make_regions(&self.region);
        for point in &self.points {
            let (id, position) = &point;
            for region in &mut self.regions {
                if self.region.contains(position.clone()) {
                    region.add(*id, position);
                }
            }
        }
        self.points.clear();
    }

    fn in_region(&self, query_area: &Rect) -> bool {
        self.region.intersect(query_area.clone()).is_some()
    }
}

#[derive(Copy, Clone)]
struct Entity {
    position: Vec2,
    bound: Circle,
}

struct Player {
    entity: Entity,
}

impl Player {
    fn new(radius: f32, position: Vec2) -> Self {
        Self {
            entity: Entity {
                position,
                bound: Circle::new(position.x, position.y, radius),
            },
        }
    }
}

struct Particle {
    entity: Entity,
    velocity: Vec2,
    acceleration: Vec2,
}

impl Particle {
    fn new(position: Vec2, radius: f32, velocity: Vec2) -> Self {
        Self {
            entity: Entity {
                position,
                bound: Circle::new(position.x, position.y, radius),
            },
            velocity,
            acceleration: Vec2::new(0.0, 500.0),
        }
    }

    fn update(&mut self, dt: f32) {
        self.velocity += self.acceleration * dt;
        self.entity.position += dt * self.velocity;
        self.entity.bound.x = self.entity.position.x;
        self.entity.bound.y = self.entity.position.y;
    }
}

trait DrawShape {
    fn draw(self: &Self) {}
}

impl DrawShape for Player {
    fn draw(&self) {
        draw_circle(
            self.entity.position.x,
            self.entity.position.y,
            self.entity.bound.r,
            RED,
        );
    }
}

impl DrawShape for Particle {
    fn draw(&self) {
        draw_circle(
            self.entity.position.x,
            self.entity.position.y,
            self.entity.bound.r,
            WHITE,
        );
    }
}

trait Movable {
    fn set_position(&mut self, position: Vec2);
}

impl Movable for Entity {
    fn set_position(&mut self, position: Vec2) {
        self.position = position;
        self.bound.x = position.x;
        self.bound.y = position.y;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Quad Tree - Demo"),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let screen_middle = Vec2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
    let mut player = Player::new(100.0, screen_middle);

    let mut debug_lines = false;
    let mut time_since_last_spawn = 0.0;

    let mut player_velocity = Vec2::ZERO;
    let mut particles = Vec::new();

    loop {
        clear_background(BLACK);
        let dt = get_frame_time();
        time_since_last_spawn += dt;

        let mut qtree = QuadNode::new(Rect::new(
            0.0,
            0.0,
            WINDOW_WIDTH as f32,
            WINDOW_HEIGHT as f32,
        ));

        let start_player_pos = player.entity.position;

        // input
        {
            let movable: &mut dyn Movable = &mut player.entity;
            let (mouse_x, mouse_y) = mouse_position();
            movable.set_position(Vec2::new(mouse_x, mouse_y));

            if is_key_pressed(KeyCode::Space) {
                debug_lines = !debug_lines;
            }

            let (_, mouse_wheel_y) = mouse_wheel();
            if mouse_wheel_y != 0.0 {
                player.entity.bound.r = (player.entity.bound.r + mouse_wheel_y * 5.0)
                    .max(30.0)
                    .min(300.0);
            }
        }

        if time_since_last_spawn >= PARTICLE_SPAWN_INTERVAL {
            particles.append(
                &mut (0..((PARTICLE_SPAWN_RATE * PARTICLE_SPAWN_INTERVAL) as i32))
                    .map(|_| {
                        Particle::new(
                            Vec2::new(rand::gen_range(0.0, WINDOW_WIDTH as f32), 0.0),
                            PARTICLE_RADIUS,
                            Vec2::new(0.0, rand::gen_range(100.0, 300.0)),
                        )
                    })
                    .collect(),
            );
            time_since_last_spawn = 0.0;
        }

        particles = particles
            .into_iter()
            .filter(|b| b.entity.position.y < WINDOW_HEIGHT as f32)
            .collect();

        for (i, particle) in particles.iter().enumerate() {
            qtree.add(i as u32, &particle.entity.position);
        }

        // collisions
        {
            let player_rect = Rect::new(
                player.entity.position.x - player.entity.bound.r,
                player.entity.position.y - player.entity.bound.r,
                player.entity.bound.r * 2.0,
                player.entity.bound.r * 2.0,
            );

            for i in qtree.query(&player_rect).iter().map(|p| p.0) {
                if particles[i as usize]
                    .entity
                    .bound
                    .overlaps(&player.entity.bound)
                {
                    let particle = &mut particles[i as usize];
                    let particle_pos: Vec2 = particle.entity.bound.point();
                    let player_pos: Vec2 = player.entity.bound.point();

                    let normal = (particle_pos - player_pos).normalize();
                    let separation_distance = particle.entity.bound.r + player.entity.bound.r;
                    particle.entity.position = player_pos + (normal * separation_distance);

                    // Calculate reflection vector using the normal
                    // v' = v - 2(v·n)n where n is the normal and v is the velocity
                    particle.velocity = (particle.velocity + player_velocity)
                        - (normal * (2.0 * particle.velocity.dot(normal)));

                    // dampening to make it more natural
                    // 0.3 = restitution coeff
                    particle.velocity = particle.velocity * 0.3;

                    // strange thingy here for a min bounce velocity thresh
                    if particle.velocity.length() < 100.0 {
                        particle.velocity = particle.velocity.normalize() * 100.0;
                    }

                    // small random variation for natural effect
                    let angle_variation: f32 = rand::gen_range(-0.1, 0.1);
                    let cos_theta = angle_variation.cos();
                    let sin_theta = angle_variation.sin();
                    let vx = particle.velocity.x * cos_theta - particle.velocity.y * sin_theta;
                    let vy = particle.velocity.x * sin_theta + particle.velocity.y * cos_theta;
                    particle.velocity = Vec2::new(vx, vy);
                }
            }
        }

        player_velocity = player.entity.position - start_player_pos;
        for p in &mut particles {
            p.update(dt);
        }

        {
            let drawable: &dyn DrawShape = &player;
            drawable.draw();
            for particle in &mut particles {
                let drawable: &dyn DrawShape = particle;
                drawable.draw();
            }
            if debug_lines {
                qtree.draw();
            }
            draw_text(
                format!("{} FPS", get_fps()).as_str(),
                WINDOW_WIDTH as f32 - 120.0,
                30.0,
                30.0,
                WHITE,
            );
            draw_text(
                "- [SPACE] to toggle quadtree debug lines",
                10.0,
                30.0,
                30.0,
                WHITE,
            );
            draw_text(
                format!("- [MOUSE_WHEEL] player radius: {}", player.entity.bound.r).as_str(),
                10.0,
                60.0,
                30.0,
                WHITE,
            );
        }

        next_frame().await;
    }
}
