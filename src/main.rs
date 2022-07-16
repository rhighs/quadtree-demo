use macroquad::{prelude::*};

use std::marker::Copy;
use std::clone::Clone;

const PLAYER_VELOCITY: f32 = 300.0;

const WINDOW_WIDTH: i32 = 1000;
const WINDOW_HEIGHT: i32 = 600;

const QUADTREE_REGION_LIMIT: usize = 10;

const BULLET_SPAWN_ITER: i32 = 100;
const BULLET_SPAWN_DELAY: f64 = 0.1;
const BULLET_RADIUS: f32 = 1.0;

trait Collidable {
    fn bounding_box(&self) -> Circle;
}

// TODO: Query with rect area instead of a point
struct QuadNode {
    limit: usize,
    region: Rect,
    points: Vec<(u32, Vec2)>,
    regions: Vec<Box<QuadNode>>
}

impl QuadNode {
    fn new(region: Rect, limit: usize) -> Self {
        Self {
            limit,
            region,
            points: Vec::new(),
            regions: Vec::new()
        }
    }

    fn make_regions(&self) -> Vec<Box<QuadNode>> {
        let x = self.region.x;
        let y = self.region.y;
        let hw = self.region.w / 2.0;
        let hh = self.region.h / 2.0;

        vec![
            Box::new(QuadNode::new(Rect::new(x, y, hw, hh), self.limit)),
            Box::new(QuadNode::new(Rect::new(x + hw, y, hw, hh), self.limit)),
            Box::new(QuadNode::new(Rect::new(x, y + hh, hw, hh), self.limit)),
            Box::new(QuadNode::new(Rect::new(x + hw, y + hh, hw, hh), self.limit)),
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
            if self.points.len() == self.limit {
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
        self.regions = self.make_regions();

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
    bouding_box: Circle,
}

impl Collidable for Entity {
    fn bounding_box(&self) -> Circle {
        self.bouding_box.clone()
    }
}

struct Player {
    entity: Entity
}

impl Player {
    fn new(radius: f32, position: Vec2) -> Self {
        Self {
            entity: Entity {
                position,
                bouding_box: Circle::new(position.x, position.y, radius)
            },
        }
    }
}

struct Bullet {
    entity: Entity,
    falling_speed: f32,
    forces: Vec<Vec2>
}

impl Bullet {
    fn new(position: Vec2, radius: f32, falling_speed: f32) -> Self {
        Self {
            entity: Entity {
                position,
                bouding_box: Circle::new(position.x, position.y, radius)
            },
            falling_speed,
            forces: Vec::new()
        }
    }

    fn fall(&mut self, tpf: f32) {
        self.entity.position.y += tpf * self.falling_speed;
        self.entity.bouding_box.y = self.entity.position.y;
    }

    // With each update, applied forces should get smaller and smaller till they get deleted from `forces`
    fn update(&mut self, tpf: f32) {
        self.fall(tpf);
        self.apply_forces(tpf);
    }

    fn register_force(&mut self, force: Vec2) {
        if force.length() > 0.01 {
            self.forces.push(force);
        }
    }

    fn apply_forces(&mut self, tpf: f32) {
        let mut n_low_forces = 0;

        for force in &mut self.forces {
            self.entity.move_by(force.clone(), tpf);
            force.x /= 1.2;
            force.y /= 1.2;

            if force.length() <= 0.01 {
                n_low_forces += 1;
            }
        }

        if n_low_forces > 0 {
            self.forces = self.forces.iter()
                .filter(|f| f.length() > 0.01)
                .copied()
                .collect();
        }
    }
}

trait DrawShape {
    fn draw(self: &Self) {}
}

impl DrawShape for Player {
    fn draw(&self) {
        draw_circle(self.entity.position.x, self.entity.position.y, self.entity.bouding_box.r, RED);
    }
}

impl DrawShape for Bullet {
    fn draw(&self) {
        draw_circle(self.entity.position.x, self.entity.position.y, self.entity.bouding_box.r, WHITE);
    }
}

trait Movable {
    fn move_by(&mut self, offset: Vec2, tpf: f32);
}

impl Movable for Entity {
    fn move_by(&mut self, offset: Vec2, tpf: f32) {
        self.position.x += offset.x * tpf * PLAYER_VELOCITY;
        self.position.y += offset.y * tpf * PLAYER_VELOCITY;

        self.bouding_box.x = self.position.x;
        self.bouding_box.y = self.position.y;
    }
}

fn try_hit(player: &Player, bullets: &Vec<Bullet>, possible_ids: Vec<u32>) -> Option<Vec<usize>> {
    let mut ids = Vec::new();

    for i in possible_ids {
        if bullets[i as usize].entity.bouding_box.overlaps(&player.entity.bouding_box) {
            ids.push(i as usize);
        }
    }

    if ids.len() > 0 {
        return Some(ids);
    }

    None
}

struct BulletSpawner {
    is_active: bool
}

impl BulletSpawner {
    fn new() -> Self {
        Self { is_active: true }
    }

    fn spawn(&mut self, no_bullets: i32, radius: f32) -> Option<Vec<Bullet>> {
        if !self.is_active {
            return None
        }

        let bullets = (0..no_bullets).into_iter()
            .map(|_| Bullet::new(
                Vec2::new(
                    rand::gen_range(0.0, WINDOW_WIDTH as f32), 
                    //rand::gen_range(0.0, WINDOW_HEIGHT as f32)),
                    0.0),
                radius,
                rand::gen_range(100.0, 300.0))
            )
            .collect();

        self.is_active = false;

        Some(bullets)
    }

    fn reset(&mut self) {
        self.is_active = true;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("QuadTree Demo"),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        high_dpi: false,
        fullscreen: false,
        sample_count: 1,
        window_resizable: false,
        icon: None,
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let screen_middle = Vec2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
    let mut bullets_in_scene = Vec::new();
    let mut bullet_spawner = BulletSpawner::new();
    let mut player = Player::new(100.0, screen_middle);

    let mut bullet_spawner_trigger_time = 0.0;

    let qregion = Rect::new(0.0, 0.0, WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
    let mut qtree = QuadNode::new(
        qregion.clone(),
        QUADTREE_REGION_LIMIT
    );

    qtree.regions = qtree.make_regions();

    // TODO: time interval based bullet spawning system
    loop {
        let start_time = get_time();
        clear_background(BLACK);
        let tpf = get_frame_time();

        if let Some(mut bullets) = bullet_spawner.spawn(BULLET_SPAWN_ITER, BULLET_RADIUS) {
            bullets_in_scene.append(&mut bullets);
        }

        for (i, bullet) in bullets_in_scene.iter().enumerate() {
            qtree.add(i as u32, &bullet.entity.position);
        }

        // Drawing 
        {
            let drawable: &dyn DrawShape = &player;
            drawable.draw();

            for bullet in &mut bullets_in_scene {
                let drawable: &dyn DrawShape = bullet;
                drawable.draw();
            }

            qtree.draw();
        }

        // Input related stuff
        {
            let movable: &mut dyn Movable = &mut player.entity;
            let mut offset = Vec2::new(0.0, 0.0);
            if is_key_down(KeyCode::Up) {
                offset.y = -1.0;
            }
            if is_key_down(KeyCode::Down) {
                offset.y = 1.0;
            }
            if is_key_down(KeyCode::Right) {
                offset.x = 1.0;
            }
            if is_key_down(KeyCode::Left) {
                offset.x = -1.0;
            }
            movable.move_by(offset, tpf);
        }

        // Handle collisition player-bullets, if a bullet gets hit bounce it back
        {
            let player_rect = Rect::new(
                player.entity.position.x - player.entity.bouding_box.r,
                player.entity.position.y - player.entity.bouding_box.r,
                player.entity.bouding_box.r * 2.0,
                player.entity.bouding_box.r * 2.0,
            );
            let ids = qtree.query(&player_rect).iter().map(|p| p.0).collect();
            let player_has_hit = try_hit(&player, &bullets_in_scene, ids);

            if let Some(hit_ids) = player_has_hit {
                for hit_id in hit_ids {
                    let hit_bullet = &mut bullets_in_scene[hit_id];
                    let bullet_pos: Vec2 = hit_bullet.entity.bouding_box.point();
                    let player_pos: Vec2 = player.entity.bouding_box.point();

                    let mut direction = bullet_pos - player_pos;
                    direction = direction.normalize() * 1 as f32;

                    hit_bullet.register_force(direction);
                }
            }

            for bullet in &mut bullets_in_scene {
                bullet.update(tpf);
            }
        }

        next_frame().await;

        bullet_spawner_trigger_time += get_time() - start_time;
        if bullet_spawner_trigger_time > BULLET_SPAWN_DELAY {
            bullet_spawner_trigger_time = 0.0;
            bullet_spawner.reset();

            bullets_in_scene = bullets_in_scene
                .into_iter()
                .filter(|b| b.entity.position.y < WINDOW_HEIGHT as f32)
                .collect();
        }
        qtree = QuadNode::new(
            qregion.clone(),
            QUADTREE_REGION_LIMIT
        );

        qtree.regions = qtree.make_regions();
    }
}
