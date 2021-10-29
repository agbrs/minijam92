#![no_std]
#![no_main]

extern crate agb;
extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use agb::{
    display::{
        background::BackgroundRegular,
        object::{ObjectControl, ObjectStandard},
        Priority, HEIGHT, WIDTH,
    },
    input::{Button, ButtonController, Tri},
    number::{FixedNum, FixedWidthSignedInteger, Rect, Vector2D},
    sound::mixer::SoundChannel,
};

agb::include_gfx!("gfx/objects.toml");
agb::include_gfx!("gfx/background.toml");

type Number = FixedNum<8>;

struct Level {
    background: BackgroundRegular<'static>,
    foreground: BackgroundRegular<'static>,
}

impl Level {
    fn load_level(
        mut backdrop: BackgroundRegular<'static>,
        mut foreground: BackgroundRegular<'static>,
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

        backdrop.commit();
        foreground.commit();

        backdrop.show();
        foreground.show();
        Self {
            background: backdrop,
            foreground,
        }
    }

    fn collides(&self, v: Vector2D<Number>) -> Option<Rect<Number>> {
        let vf = v.floor();
        let (x, y) = (vf / 8).get();
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

    enemies: Vec<Enemy<'a>>,
}

struct Entity<'a> {
    sprite: ObjectStandard<'a>,
    position: Vector2D<Number>,
    velocity: Vector2D<Number>,
    collision_mask: Rect<u16>,
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
        }
    }

    fn update_position(&mut self, level: &Level) -> Vector2D<Number> {
        let initial_position = self.position;

        let y = self.velocity.y.to_raw().signum();
        if y != 0 {
            let delta = self.collision_in_direction((0, y).into(), self.velocity.y.abs(), |v| {
                level.collides(v)
            });
            self.position += delta;
        }
        let x = self.velocity.x.to_raw().signum();
        if x != 0 {
            let delta = self.collision_in_direction((x, 0).into(), self.velocity.x.abs(), |v| {
                level.collides(v)
            });
            self.position += delta;
        }

        self.position - initial_position
    }

    fn collision_in_direction(
        &mut self,
        direction: Vector2D<Number>,
        distance: Number,
        collision: impl Fn(Vector2D<Number>) -> Option<Rect<Number>>,
    ) -> Vector2D<Number> {
        let number_collision: Rect<Number> = Rect::new(
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
        let collider_center = self.position + number_collision.position;

        let center_collision_point: Vector2D<Number> =
            collider_center + number_collision.size.hadamard(direction) / 2;

        let direction_transpose: Vector2D<Number> = direction.swap();
        let triple_collider: [Vector2D<Number>; 2] = [
            center_collision_point + number_collision.size.hadamard(direction_transpose) * 28 / 64,
            center_collision_point - number_collision.size.hadamard(direction_transpose) * 28 / 64,
        ];

        let original_distance = direction * distance;
        let mut final_distance = original_distance;

        for edge_points in triple_collider {
            let point = edge_points + original_distance;
            if let Some(collider) = collision(point) {
                let center = collider.position + collider.size / 2;
                let edge = center - collider.size.hadamard(direction) / 2;
                final_distance = original_distance
                    - (point - edge).hadamard((direction.x.abs(), direction.y.abs()).into());
                self.velocity = self.velocity.hadamard(direction_transpose);
            }
        }

        final_distance
    }

    fn commit_with_fudge(&mut self, offset: Vector2D<Number>, fudge: Vector2D<i32>) {
        let position = (self.position - offset).floor() + fudge;
        self.sprite.set_position(position - (8, 8).into());
        if position.x < -8 || position.x > WIDTH + 8 || position.y < -8 || position.y > HEIGHT + 8 {
            self.sprite.hide();
        } else {
            self.sprite.show();
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
}

impl SwordState {
    fn attack_duration(self) -> u16 {
        match self {
            SwordState::LongSword => 60,
        }
    }
    fn jump_attack_duration(self) -> u16 {
        match self {
            SwordState::LongSword => 34,
        }
    }
    fn attack_frame(self, timer: u16) -> u16 {
        match self {
            SwordState::LongSword => (self.attack_duration() - timer) / 8,
        }
    }
    fn jump_attack_frame(self, timer: u16) -> u16 {
        match self {
            SwordState::LongSword => (self.jump_attack_duration() - timer) / 8,
        }
    }
    fn hold_frame(self) -> u16 {
        match self {
            SwordState::LongSword => 7,
        }
    }
    fn jump_attack_hold_frame(self) -> u16 {
        match self {
            SwordState::LongSword => 13,
        }
    }

    fn cooldown_time(self) -> u16 {
        match self {
            SwordState::LongSword => 20,
        }
    }
    fn to_sprite_id(self, frame: u16) -> u16 {
        match self {
            SwordState::LongSword => (16 + frame) * 4,
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
        }
    }
    fn fudge(self, frame: u16) -> i32 {
        match self {
            SwordState::LongSword => long_sword_fudge(frame),
        }
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
}

impl<'a> Player<'a> {
    fn new(object_controller: &'a ObjectControl) -> Player {
        let mut entity = Entity::new(
            object_controller,
            Rect::new((0_u16, 0_u16).into(), (8_u16, 10_u16).into()),
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
            sword: SwordState::LongSword,
            sprite_offset: 0,
            attack_timer: AttackTimer::Idle,
            fudge_factor: (0, 0).into(),
        }
    }

    fn update(&mut self, buttons: &ButtonController, level: &Level) {
        let x = buttons.x_tri();

        self.fudge_factor = (0, 0).into();

        match self.state {
            PlayerState::OnGround => {
                self.entity.velocity.y = 0.into();

                match &mut self.attack_timer {
                    AttackTimer::Idle => {
                        if x != Tri::Zero {
                            self.facing = x;
                        }
                        self.entity.sprite.set_hflip(self.facing == Tri::Negative);
                        self.entity.velocity.x += Number::new(x as i32) / 4;
                        if self.entity.velocity.x.abs() > Number::new(1) / 10 {
                            if self.sprite_offset >= 6 * 4 {
                                self.sprite_offset = 0;
                            }

                            self.entity
                                .sprite
                                .set_tile_id((4 + self.sprite_offset / 4) * 4);
                        } else {
                            if self.sprite_offset >= 4 * 8 {
                                self.sprite_offset = 0;
                            }

                            self.entity
                                .sprite
                                .set_tile_id((0 + self.sprite_offset / 8) * 4);
                        }

                        if buttons.is_just_pressed(Button::B) {
                            self.attack_timer = AttackTimer::Attack(self.sword.attack_duration());
                        } else if buttons.is_just_pressed(Button::A) {
                            self.entity.velocity.y -= 2;
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
                let gravity: Number = 1.into();
                let gravity = gravity / 16;
                self.entity.velocity.y += gravity;

                match &mut self.attack_timer {
                    AttackTimer::Idle => {
                        if self.sprite_offset < 3 * 4 {
                            self.entity
                                .sprite
                                .set_tile_id((10 + self.sprite_offset / 4) * 4);
                        } else if self.entity.velocity.y.abs() < Number::new(1) / 5 {
                            self.entity.sprite.set_tile_id(13 * 4);
                        } else if self.entity.velocity.y > 1.into() {
                            self.entity.sprite.set_tile_id(15 * 4);
                        } else if self.entity.velocity.y > 0.into() {
                            self.entity.sprite.set_tile_id(14 * 4);
                        } else {
                            self.entity.sprite.set_tile_id(12 * 4);
                        }

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
        self.entity.velocity.x = self.entity.velocity.x * 40 / 64;

        self.entity.update_position(level);

        if self
            .entity
            .collision_in_direction((0, 1).into(), 1.into(), |v| level.collides(v))
            .y
            .abs()
            < 1.into()
        {
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
}

enum SlimeState {
    Idle,
    Chasing,
}

struct SlimeData {
    sprite_offset: u16,
    slime_state: SlimeState,
}

impl SlimeData {
    fn new() -> Self {
        Self {
            sprite_offset: 0,
            slime_state: SlimeState::Idle,
        }
    }

    fn update(&mut self, entity: &mut Entity, _player: &Player, _level: &Level) {
        match self.slime_state {
            SlimeState::Idle => {
                self.sprite_offset += 1;
                if self.sprite_offset > 16 {
                    self.sprite_offset = 0;
                }

                entity.sprite.set_tile_id((29 + self.sprite_offset / 8) * 4);
            }
            SlimeState::Chasing => todo!(),
        }
    }
}

impl EnemyData {
    fn collision_mask(&self) -> Rect<u16> {
        match self {
            &EnemyData::Slime(_) => Rect::new((0u16, 0u16).into(), (4u16, 11u16).into()),
        }
    }

    fn tile_id(&self) -> u16 {
        match self {
            &EnemyData::Slime(_) => 29,
        }
    }

    fn update(&mut self, entity: &mut Entity, player: &Player, level: &Level) {
        match self {
            EnemyData::Slime(data) => data.update(entity, player, level),
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

    fn update(&mut self, player: &Player, level: &Level) {
        self.enemy_data.update(&mut self.entity, player, level);
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

        for enemy in self.enemies.iter_mut() {
            enemy.update(&self.player, &self.level);
            enemy.entity.commit_with_fudge((0, 0).into(), (0, 0).into());
        }

        self.frame_count += 1;
        GameStatus::Continue
    }

    fn new(object: &'a ObjectControl, level: Level) -> Self {
        let mut slime = Enemy::new(object, EnemyData::Slime(SlimeData::new()));
        slime.entity.position = (100, 50).into();

        Self {
            player: Player::new(object),
            input: ButtonController::new(),
            frame_count: 0,
            level,

            enemies: vec![slime],
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
