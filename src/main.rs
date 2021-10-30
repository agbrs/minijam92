#![no_std]
#![no_main]

extern crate agb;
extern crate alloc;

use alloc::vec::Vec;

use agb::{
    display::{
        background::BackgroundRegular,
        object::{ObjectControl, ObjectStandard},
        Priority, HEIGHT, WIDTH,
    },
    input::{Button, ButtonController, Tri},
    number::{FixedNum, Rect, Vector2D},
    sound::mixer::SoundChannel,
};
use generational_arena::Arena;

agb::include_gfx!("gfx/objects.toml");
agb::include_gfx!("gfx/background.toml");

type Number = FixedNum<8>;

struct Level {
    background: BackgroundRegular<'static>,
    foreground: BackgroundRegular<'static>,
    clouds: BackgroundRegular<'static>,

    slime_spawns: Vec<(u16, u16)>,
    bat_spawns: Vec<(u16, u16)>,
}

impl Level {
    fn load_level(
        mut backdrop: BackgroundRegular<'static>,
        mut foreground: BackgroundRegular<'static>,
        mut clouds: BackgroundRegular<'static>,
    ) -> Self {
        backdrop.set_position(Vector2D::new(0, 0));
        backdrop.set_map(agb::display::background::Map::new(
            tilemap::BACKGROUND_MAP,
            Vector2D::new(tilemap::WIDTH, tilemap::HEIGHT),
            0,
        ));
        backdrop.set_priority(Priority::P0);

        foreground.set_position(Vector2D::new(0, 0));
        foreground.set_map(agb::display::background::Map::new(
            tilemap::FOREGROUND_MAP,
            Vector2D::new(tilemap::WIDTH, tilemap::HEIGHT),
            0,
        ));
        foreground.set_priority(Priority::P2);

        clouds.set_position(Vector2D::new(0, -5));
        clouds.set_map(agb::display::background::Map::new(
            tilemap::CLOUD_MAP,
            Vector2D::new(tilemap::WIDTH, tilemap::HEIGHT),
            0,
        ));
        clouds.set_priority(Priority::P3);

        backdrop.commit();
        foreground.commit();
        clouds.commit();

        backdrop.show();
        foreground.show();
        clouds.show();

        let slime_spawns = tilemap::SLIME_SPAWNS_X
            .iter()
            .enumerate()
            .map(|(i, x)| (*x, tilemap::SLIME_SPAWNS_Y[i]))
            .collect();

        let bat_spawns = tilemap::BAT_SPAWNS_X
            .iter()
            .enumerate()
            .map(|(i, x)| (*x, tilemap::BAT_SPAWNS_Y[i]))
            .collect();

        Self {
            background: backdrop,
            foreground,
            clouds,

            slime_spawns,
            bat_spawns,
        }
    }

    fn collides(&self, v: Vector2D<Number>) -> Option<Rect<Number>> {
        let factor: Number = Number::new(1) / Number::new(8);
        let (x, y) = (v * factor).floor().get();

        if (x < 0 || x > tilemap::WIDTH as i32) || (y < 0 || y > tilemap::HEIGHT as i32) {
            return Some(Rect::new((x * 8, y * 8).into(), (8, 8).into()));
        }
        let position = tilemap::WIDTH as usize * y as usize + x as usize;
        let tile_foreground = tilemap::FOREGROUND_MAP[position];
        let tile_background = tilemap::BACKGROUND_MAP[position];
        let tile_foreground_property = tilemap::TILE_TYPES[tile_foreground as usize];
        let tile_background_property = tilemap::TILE_TYPES[tile_background as usize];

        if tile_foreground_property == 1 || tile_background_property == 1 {
            Some(Rect::new((x * 8, y * 8).into(), (8, 8).into()))
        } else {
            None
        }
    }
}

struct Game<'a> {
    player: Player<'a>,
    input: ButtonController,
    frame_count: u32,
    level: Level,

    enemies: Arena<Enemy<'a>>,
}

struct Entity<'a> {
    sprite: ObjectStandard<'a>,
    position: Vector2D<Number>,
    velocity: Vector2D<Number>,
    collision_mask: Rect<u16>,
    visible: bool,
}

impl<'a> Entity<'a> {
    fn new(object_controller: &'a ObjectControl, collision_mask: Rect<u16>) -> Self {
        let mut sprite = object_controller.get_object_standard();
        sprite.set_priority(Priority::P1);
        Entity {
            sprite,
            collision_mask,
            position: (0, 0).into(),
            velocity: (0, 0).into(),
            visible: true,
        }
    }

    fn update_position(&mut self, level: &Level) -> Vector2D<Number> {
        let initial_position = self.position;

        let y = self.velocity.y.to_raw().signum();
        if y != 0 {
            let (delta, collided) =
                self.collision_in_direction((0, y).into(), self.velocity.y.abs(), |v| {
                    level.collides(v)
                });
            self.position += delta;
            if collided {
                self.velocity.y = 0.into();
            }
        }
        let x = self.velocity.x.to_raw().signum();
        if x != 0 {
            let (delta, collided) =
                self.collision_in_direction((x, 0).into(), self.velocity.x.abs(), |v| {
                    level.collides(v)
                });
            self.position += delta;
            if collided {
                self.velocity.x = 0.into();
            }
        }

        self.position - initial_position
    }

    fn collider(&self) -> Rect<Number> {
        let mut number_collision: Rect<Number> = Rect::new(
            (
                self.collision_mask.position.x as i32,
                self.collision_mask.position.y as i32,
            )
                .into(),
            (
                self.collision_mask.size.x as i32,
                self.collision_mask.size.y as i32,
            )
                .into(),
        );
        number_collision.position = self.position + number_collision.position;
        number_collision
    }

    fn collision_in_direction(
        &mut self,
        direction: Vector2D<Number>,
        distance: Number,
        collision: impl Fn(Vector2D<Number>) -> Option<Rect<Number>>,
    ) -> (Vector2D<Number>, bool) {
        let number_collision = self.collider();

        let center_collision_point: Vector2D<Number> =
            number_collision.position + number_collision.size.hadamard(direction) / 2;

        let direction_transpose: Vector2D<Number> = direction.swap();
        let small = direction_transpose * Number::new(4) / 64;
        let triple_collider: [Vector2D<Number>; 2] = [
            center_collision_point + number_collision.size.hadamard(direction_transpose) / 2
                - small,
            center_collision_point - number_collision.size.hadamard(direction_transpose) / 2
                + small,
        ];

        let original_distance = direction * distance;
        let mut final_distance = original_distance;

        let mut has_collided = false;

        for edge_point in triple_collider {
            let point = edge_point + original_distance;
            if let Some(collider) = collision(point) {
                let center = collider.position + collider.size / 2;
                let edge = center - collider.size.hadamard(direction) / 2;
                let new_distance = (edge - center_collision_point)
                    .hadamard((direction.x.abs(), direction.y.abs()).into());
                if final_distance.manhattan_distance() > new_distance.manhattan_distance() {
                    final_distance = new_distance;
                }
                has_collided = true;
            }
        }

        (final_distance, has_collided)
    }

    fn commit_with_fudge(&mut self, offset: Vector2D<Number>, fudge: Vector2D<i32>) {
        if !self.visible {
            self.sprite.hide();
        } else {
            let position = (self.position - offset).floor() + fudge;
            self.sprite.set_position(position - (8, 8).into());
            if position.x < -8
                || position.x > WIDTH + 8
                || position.y < -8
                || position.y > HEIGHT + 8
            {
                self.sprite.hide();
            } else {
                self.sprite.show();
            }
        }
        self.sprite.commit();
    }
}

#[derive(PartialEq, Eq)]
enum PlayerState {
    OnGround,
    InAir,
}

#[derive(Clone, Copy)]
enum SwordState {
    LongSword,
    ShortSword,
}

impl SwordState {
    fn ground_walk_force(self) -> Number {
        match self {
            SwordState::LongSword => Number::new(4) / 16,
            SwordState::ShortSword => Number::new(5) / 16,
        }
    }
    fn jump_impulse(self) -> Number {
        match self {
            SwordState::LongSword => Number::new(32) / 16,
            SwordState::ShortSword => Number::new(35) / 16,
        }
    }
    fn air_move_force(self) -> Number {
        match self {
            SwordState::LongSword => Number::new(4) / 256,
            SwordState::ShortSword => Number::new(5) / 256,
        }
    }
    fn idle_animation(self, counter: &mut u16) -> u16 {
        match self {
            SwordState::LongSword => {
                if *counter >= 4 * 8 {
                    *counter = 0;
                }
                (0 + *counter / 8) * 4
            }
            SwordState::ShortSword => {
                if *counter >= 4 * 8 {
                    *counter = 0;
                }
                (41 + *counter / 8) * 4
            }
        }
    }
    fn jump_offset(self) -> u16 {
        match self {
            SwordState::LongSword => 10,
            SwordState::ShortSword => 51,
        }
    }
    fn walk_animation(self, counter: &mut u16) -> u16 {
        match self {
            SwordState::LongSword => {
                if *counter >= 6 * 4 {
                    *counter = 0;
                }
                (4 + *counter / 4) * 4
            }
            SwordState::ShortSword => {
                if *counter >= 6 * 4 {
                    *counter = 0;
                }
                (45 + *counter / 4) * 4
            }
        }
    }
    fn attack_duration(self) -> u16 {
        match self {
            SwordState::LongSword => 60,
            SwordState::ShortSword => 40,
        }
    }
    fn jump_attack_duration(self) -> u16 {
        match self {
            SwordState::LongSword => 34,
            SwordState::ShortSword => 28,
        }
    }
    fn attack_frame(self, timer: u16) -> u16 {
        match self {
            SwordState::LongSword => (self.attack_duration() - timer) / 8,
            SwordState::ShortSword => (self.attack_duration() - timer) / 8,
        }
    }
    fn jump_attack_frame(self, timer: u16) -> u16 {
        match self {
            SwordState::LongSword => (self.jump_attack_duration() - timer) / 8,
            SwordState::ShortSword => (self.jump_attack_duration() - timer) / 8,
        }
    }
    fn hold_frame(self) -> u16 {
        match self {
            SwordState::LongSword => 7,
            SwordState::ShortSword => 7,
        }
    }
    fn jump_attack_hold_frame(self) -> u16 {
        match self {
            SwordState::LongSword => 13,
            SwordState::ShortSword => 54,
        }
    }

    fn cooldown_time(self) -> u16 {
        match self {
            SwordState::LongSword => 20,
            SwordState::ShortSword => 10,
        }
    }
    fn to_sprite_id(self, frame: u16) -> u16 {
        match self {
            SwordState::LongSword => (16 + frame) * 4,
            SwordState::ShortSword => (57 + frame) * 4,
        }
    }
    fn to_jump_sprite_id(self, frame: u16) -> u16 {
        match self {
            SwordState::LongSword => {
                if frame == self.jump_attack_hold_frame() {
                    frame * 4
                } else {
                    (24 + frame) * 4
                }
            }
            SwordState::ShortSword => {
                if frame == self.jump_attack_hold_frame() {
                    frame * 4
                } else {
                    (65 + frame) * 4
                }
            }
        }
    }
    fn fudge(self, frame: u16) -> i32 {
        match self {
            SwordState::LongSword => long_sword_fudge(frame),
            SwordState::ShortSword => short_sword_fudge(frame),
        }
    }
    // origin at top left pre fudge boxes
    fn ground_attack_hurtbox(self, frame: u16) -> Option<Rect<Number>> {
        match self {
            SwordState::LongSword => long_sword_hurtbox(frame),
            SwordState::ShortSword => short_sword_hurtbox(frame),
        }
    }
    fn air_attack_hurtbox(self, frame: u16) -> Option<Rect<Number>> {
        Some(Rect::new((2, 2).into(), (12, 12).into()))
    }
}

fn long_sword_hurtbox(frame: u16) -> Option<Rect<Number>> {
    match frame {
        0 => Some(Rect::new((1, 10).into(), (6, 3).into())),
        1 => Some(Rect::new((0, 9).into(), (7, 2).into())),
        2 => Some(Rect::new((0, 1).into(), (6, 8).into())),
        3 => Some(Rect::new((3, 0).into(), (6, 8).into())),
        4 => Some(Rect::new((6, 3).into(), (10, 8).into())),
        5 => Some(Rect::new((6, 5).into(), (10, 9).into())),
        6 => Some(Rect::new((6, 5).into(), (10, 9).into())),
        7 => Some(Rect::new((6, 5).into(), (10, 9).into())),
        _ => unreachable!(),
    }
}

fn short_sword_hurtbox(frame: u16) -> Option<Rect<Number>> {
    match frame {
        0 => None,
        1 => Some(Rect::new((10, 5).into(), (3, 5).into())),
        2 => Some(Rect::new((8, 5).into(), (6, 6).into())),
        3 => Some(Rect::new((8, 6).into(), (8, 8).into())),
        4 => Some(Rect::new((8, 7).into(), (5, 7).into())),
        5 => Some(Rect::new((8, 7).into(), (7, 7).into())),
        6 => Some(Rect::new((8, 5).into(), (7, 8).into())),
        7 => Some(Rect::new((8, 4).into(), (4, 7).into())),
        _ => unreachable!(),
    }
}

fn short_sword_fudge(frame: u16) -> i32 {
    match frame {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 3,
        5 => 3,
        6 => 3,
        7 => 3,
        _ => unreachable!(),
    }
}

fn long_sword_fudge(frame: u16) -> i32 {
    match frame {
        0 => 0,
        1 => 0,
        2 => 1,
        3 => 4,
        4 => 5,
        5 => 5,
        6 => 5,
        7 => 4,
        _ => unreachable!(),
    }
}

enum AttackTimer {
    Idle,
    Attack(u16),
    Cooldown(u16),
}

struct Player<'a> {
    entity: Entity<'a>,
    facing: Tri,
    state: PlayerState,
    sprite_offset: u16,
    attack_timer: AttackTimer,
    sword: SwordState,
    fudge_factor: Vector2D<i32>,
    hurtbox: Option<Rect<Number>>,
}

impl<'a> Player<'a> {
    fn new(object_controller: &'a ObjectControl) -> Player {
        let mut entity = Entity::new(
            object_controller,
            Rect::new((0_u16, 0_u16).into(), (8_u16, 12_u16).into()),
        );
        entity
            .sprite
            .set_sprite_size(agb::display::object::Size::S16x16);
        entity.sprite.set_tile_id(0);
        entity.sprite.show();
        entity.position = (58, 26).into();
        entity.sprite.commit();

        Player {
            entity,
            facing: Tri::Zero,
            state: PlayerState::OnGround,
            sword: SwordState::ShortSword,
            sprite_offset: 0,
            attack_timer: AttackTimer::Idle,
            fudge_factor: (0, 0).into(),
            hurtbox: None,
        }
    }

    fn update(&mut self, buttons: &ButtonController, level: &Level) {
        let x = buttons.x_tri();

        self.fudge_factor = (0, 0).into();
        let mut hurtbox = None;

        match self.state {
            PlayerState::OnGround => {
                self.entity.velocity.y = 0.into();
                self.entity.velocity.x = self.entity.velocity.x * 40 / 64;

                match &mut self.attack_timer {
                    AttackTimer::Idle => {
                        if x != Tri::Zero {
                            self.facing = x;
                        }
                        self.entity.sprite.set_hflip(self.facing == Tri::Negative);
                        self.entity.velocity.x += self.sword.ground_walk_force() * x as i32;
                        if self.entity.velocity.x.abs() > Number::new(1) / 10 {
                            self.entity
                                .sprite
                                .set_tile_id(self.sword.walk_animation(&mut self.sprite_offset));
                        } else {
                            self.entity
                                .sprite
                                .set_tile_id(self.sword.idle_animation(&mut self.sprite_offset));
                        }

                        if buttons.is_just_pressed(Button::B) {
                            self.attack_timer = AttackTimer::Attack(self.sword.attack_duration());
                        } else if buttons.is_just_pressed(Button::A) {
                            self.entity.velocity.y -= self.sword.jump_impulse();
                            self.state = PlayerState::InAir;
                            self.sprite_offset = 0;
                        }
                    }
                    AttackTimer::Attack(a) => {
                        *a -= 1;
                        let frame = self.sword.attack_frame(*a);
                        self.fudge_factor.x = self.sword.fudge(frame) * self.facing as i32;
                        self.entity
                            .sprite
                            .set_tile_id(self.sword.to_sprite_id(frame));

                        hurtbox = self.sword.ground_attack_hurtbox(frame);

                        if *a == 0 {
                            self.attack_timer = AttackTimer::Cooldown(self.sword.cooldown_time());
                        }
                    }
                    AttackTimer::Cooldown(a) => {
                        *a -= 1;
                        let frame = self.sword.hold_frame();
                        self.fudge_factor.x = self.sword.fudge(frame) * self.facing as i32;
                        self.entity
                            .sprite
                            .set_tile_id(self.sword.to_sprite_id(frame));
                        if *a == 0 {
                            self.attack_timer = AttackTimer::Idle;
                        }
                    }
                }
            }
            PlayerState::InAir => {
                self.entity.velocity.x = self.entity.velocity.x * 63 / 64;

                match &mut self.attack_timer {
                    AttackTimer::Idle => {
                        let sprite = if self.sprite_offset < 3 * 4 {
                            self.sprite_offset / 4
                        } else if self.entity.velocity.y.abs() < Number::new(1) / 5 {
                            3
                        } else if self.entity.velocity.y > 1.into() {
                            5
                        } else if self.entity.velocity.y > 0.into() {
                            4
                        } else {
                            2
                        };
                        self.entity
                            .sprite
                            .set_tile_id((sprite + self.sword.jump_offset()) * 4);

                        if x != Tri::Zero {
                            self.facing = x;
                        }
                        self.entity.sprite.set_hflip(self.facing == Tri::Negative);
                        self.entity.velocity.x += self.sword.air_move_force() * x as i32;

                        if buttons.is_just_pressed(Button::B) {
                            self.attack_timer =
                                AttackTimer::Attack(self.sword.jump_attack_duration());
                        }
                    }
                    AttackTimer::Attack(a) => {
                        *a -= 1;
                        let frame = self.sword.jump_attack_frame(*a);
                        self.entity
                            .sprite
                            .set_tile_id(self.sword.to_jump_sprite_id(frame));

                        hurtbox = self.sword.air_attack_hurtbox(frame);

                        if *a == 0 {
                            self.attack_timer = AttackTimer::Idle;
                        }
                    }
                    AttackTimer::Cooldown(_) => {
                        self.attack_timer = AttackTimer::Idle;
                    }
                }
            }
        }
        let gravity: Number = 1.into();
        let gravity = gravity / 16;
        self.entity.velocity.y += gravity;

        let fudge_number = (self.fudge_factor.x, self.fudge_factor.y).into();

        // convert the hurtbox to a location in the game
        self.hurtbox = hurtbox.map(|h| {
            let mut b = Rect::new(h.position - (8, 8).into(), h.size);
            if self.facing == Tri::Negative {
                b.position.x = -b.position.x - b.size.x;
            }
            b.position += self.entity.position + fudge_number;
            b
        });

        self.entity.update_position(level);
        let (_, collided_down) = self
            .entity
            .collision_in_direction((0, 1).into(), 1.into(), |v| level.collides(v));

        if collided_down {
            self.state = PlayerState::OnGround;
        } else {
            self.state = PlayerState::InAir;
        }

        self.sprite_offset += 1;
    }

    fn commit(&mut self, offset: Vector2D<Number>) {
        self.entity.commit_with_fudge(offset, self.fudge_factor);
    }
}

enum EnemyData {
    Slime(SlimeData),
    Bat(BatData),
}

struct BatData {
    sprite_offset: u16,
    bat_state: BatState,
}

enum BatState {
    Idle,
    Chasing(u16),
    Dead,
}

struct SlimeData {
    sprite_offset: u16,
    slime_state: SlimeState,
}

impl BatData {
    fn new() -> Self {
        Self {
            sprite_offset: 0,
            bat_state: BatState::Idle,
        }
    }

    fn update(&mut self, entity: &mut Entity, player: &Player, level: &Level) -> EnemyInstruction {
        let should_die = player
            .hurtbox
            .as_ref()
            .map(|hurtbox| hurtbox.touches(entity.collider()))
            .unwrap_or(false);

        match &mut self.bat_state {
            BatState::Idle => {
                self.sprite_offset += 1;
                if self.sprite_offset >= 9 * 8 {
                    self.sprite_offset = 0;
                }

                entity.sprite.set_tile_id((78 + self.sprite_offset / 8) * 4);

                if (entity.position - player.entity.position).manhattan_distance() < 50.into() {
                    self.bat_state = BatState::Chasing(300);
                    self.sprite_offset /= 4;
                }

                if should_die {
                    self.bat_state = BatState::Dead;
                }
            }
            BatState::Chasing(count) => {
                self.sprite_offset += 1;

                let speed = Number::new(1) / Number::new(4);
                let target_velocity = (player.entity.position - entity.position);
                entity.velocity = target_velocity.normalise() * speed;

                if self.sprite_offset >= 9 * 2 {
                    self.sprite_offset = 0;
                }
                entity.sprite.set_tile_id((78 + self.sprite_offset / 2) * 4);

                entity.update_position(level);

                if *count == 0 {
                    self.bat_state = BatState::Idle;
                    self.sprite_offset *= 4;
                } else {
                    *count -= 1;
                }

                if should_die {
                    self.bat_state = BatState::Dead;
                }
            }
            BatState::Dead => {
                entity.sprite.set_tile_id(87 * 4);
                let gravity: Number = 1.into();
                let gravity = gravity / 16;
                entity.velocity.x = 0.into();
                entity.velocity.y += gravity;

                entity.update_position(level);
            }
        }
        EnemyInstruction::None
    }
}

enum SlimeState {
    Idle,
    Chasing(Tri),
    Dead(u16),
}

impl SlimeData {
    fn new() -> Self {
        Self {
            sprite_offset: 0,
            slime_state: SlimeState::Idle,
        }
    }

    fn update(&mut self, entity: &mut Entity, player: &Player, level: &Level) -> EnemyInstruction {
        match &mut self.slime_state {
            SlimeState::Idle => {
                self.sprite_offset += 1;
                if self.sprite_offset >= 32 {
                    self.sprite_offset = 0;
                }

                entity
                    .sprite
                    .set_tile_id((29 + self.sprite_offset / 16) * 4);

                if (player.entity.position - entity.position).manhattan_distance() < 40.into() {
                    let direction = if player.entity.position.x > entity.position.x {
                        Tri::Positive
                    } else if player.entity.position.x < entity.position.x {
                        Tri::Negative
                    } else {
                        Tri::Zero
                    };

                    self.slime_state = SlimeState::Chasing(direction);
                    self.sprite_offset = 0;
                }
                if let Some(hurtbox) = &player.hurtbox {
                    if hurtbox.touches(entity.collider()) {
                        self.slime_state = SlimeState::Dead(0);
                    }
                }
            }
            SlimeState::Chasing(direction) => {
                self.sprite_offset += 1;
                if self.sprite_offset >= 7 * 6 {
                    self.slime_state = SlimeState::Idle;
                } else {
                    let frame = ping_pong(self.sprite_offset / 6, 5);
                    entity.sprite.set_tile_id((frame + 31) * 4);

                    entity.velocity.x = match frame {
                        2 | 3 | 4 => {
                            (Number::new(1) / 5)
                                * match direction {
                                    Tri::Negative => -1,
                                    Tri::Positive => 1,
                                    Tri::Zero => 0,
                                }
                        }
                        _ => 0.into(),
                    };

                    let gravity: Number = 1.into();
                    let gravity = gravity / 16;
                    entity.velocity.y += gravity;

                    let updated_position = entity.update_position(level);
                    if updated_position.y > 0.into() && self.sprite_offset > 2 * 6 {
                        // we're falling
                        self.sprite_offset = 6 * 6;
                    }
                }
                if let Some(hurtbox) = &player.hurtbox {
                    if hurtbox.touches(entity.collider()) {
                        self.slime_state = SlimeState::Dead(0);
                    }
                }
            }
            SlimeState::Dead(count) => {
                if *count < 5 * 4 {
                    entity.sprite.set_tile_id((36 + *count / 4) * 4);
                    *count += 1;
                } else {
                    return EnemyInstruction::Remove;
                }
            }
        }
        EnemyInstruction::None
    }
}

enum EnemyInstruction {
    None,
    Remove,
    DamagePlayer,
}

impl EnemyData {
    fn collision_mask(&self) -> Rect<u16> {
        match self {
            EnemyData::Slime(_) => Rect::new((0u16, 0u16).into(), (4u16, 11u16).into()),
            EnemyData::Bat(_) => Rect::new((1u16, 0u16).into(), (15u16, 6u16).into()),
        }
    }

    fn tile_id(&self) -> u16 {
        match self {
            EnemyData::Slime(_) => 29,
            EnemyData::Bat(_) => 78,
        }
    }

    fn update(&mut self, entity: &mut Entity, player: &Player, level: &Level) -> EnemyInstruction {
        match self {
            EnemyData::Slime(data) => data.update(entity, player, level),
            EnemyData::Bat(data) => data.update(entity, player, level),
        }
    }
}

struct Enemy<'a> {
    entity: Entity<'a>,
    enemy_data: EnemyData,
}

impl<'a> Enemy<'a> {
    fn new(object_controller: &'a ObjectControl, enemy_data: EnemyData) -> Self {
        let mut entity = Entity::new(object_controller, enemy_data.collision_mask());

        entity
            .sprite
            .set_sprite_size(agb::display::object::Size::S16x16);
        entity.sprite.set_tile_id(enemy_data.tile_id());
        entity.sprite.show();

        entity.sprite.commit();

        Self { entity, enemy_data }
    }

    fn update(&mut self, player: &Player, level: &Level) -> EnemyInstruction {
        self.enemy_data.update(&mut self.entity, player, level)
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum GameStatus {
    Continue,
    Lost,
    Won,
}

impl<'a> Game<'a> {
    fn advance_frame(&mut self) -> GameStatus {
        self.input.update();
        self.player.update(&self.input, &self.level);
        self.player.commit((0, 0).into());

        let mut remove = Vec::with_capacity(10);

        for (idx, enemy) in self.enemies.iter_mut() {
            match enemy.update(&self.player, &self.level) {
                EnemyInstruction::Remove => {
                    remove.push(idx);
                }
                EnemyInstruction::None => {}
                EnemyInstruction::DamagePlayer => {}
            }
            enemy.entity.commit_with_fudge((0, 0).into(), (0, 0).into());
        }

        for i in remove {
            self.enemies.remove(i);
        }

        self.frame_count += 1;
        GameStatus::Continue
    }

    fn new(object: &'a ObjectControl, level: Level) -> Self {
        let mut enemies = Arena::with_capacity(100);
        for slime in level.slime_spawns.iter().map(|slime_spawn| {
            let mut slime = Enemy::new(object, EnemyData::Slime(SlimeData::new()));
            slime.entity.position = (slime_spawn.0 as i32, slime_spawn.1 as i32 - 7).into();
            slime
        }) {
            enemies.insert(slime);
        }

        for bat in level.bat_spawns.iter().map(|bat_spawn| {
            let mut bat = Enemy::new(object, EnemyData::Bat(BatData::new()));
            bat.entity.position = (bat_spawn.0 as i32, bat_spawn.1 as i32).into();
            bat
        }) {
            enemies.insert(bat);
        }

        Self {
            player: Player::new(object),
            input: ButtonController::new(),
            frame_count: 0,
            level,

            enemies,
        }
    }
}

const MINIMUSIC: &[u8] = agb::include_wav!("sfx/01_-_The_Purple_Night.wav");

fn game_with_level(gba: &mut agb::Gba) {
    let mut object = gba.display.object.get();
    object.set_sprite_palettes(objects::objects.palettes);
    object.set_sprite_tilemap(objects::objects.tiles);

    let mut background = gba.display.video.tiled0();

    background.set_background_palettes(background::background.palettes);
    background.set_background_tilemap(0, background::background.tiles);

    object.enable();
    let object = object;

    let vblank = agb::interrupt::VBlank::get();
    vblank.wait_for_vblank();

    let mut game = Game::new(
        &object,
        Level::load_level(
            background.get_regular().unwrap(),
            background.get_regular().unwrap(),
            background.get_regular().unwrap(),
        ),
    );

    let mut mixer = gba.mixer.mixer();
    mixer.enable();
    let mut channel = SoundChannel::new(MINIMUSIC);
    channel.stereo().should_loop();
    mixer.play_sound(channel).unwrap();
    mixer.vblank();

    loop {
        vblank.wait_for_vblank();
        mixer.vblank();
        game.advance_frame();
    }
}

mod tilemap {
    include!(concat!(env!("OUT_DIR"), "/tilemap.rs"));
}

#[agb::entry]
fn main() -> ! {
    let mut gba = agb::Gba::new();

    game_with_level(&mut gba);

    loop {}
}

fn ping_pong(i: u16, n: u16) -> u16 {
    let cycle = 2 * (n - 1);
    let i = i % cycle;
    if i >= n {
        cycle - i
    } else {
        i
    }
}
