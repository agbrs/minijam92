#![no_std]
#![no_main]

use agb::{
    display::{
        object::{ObjectControl, ObjectStandard},
        Priority, HEIGHT, WIDTH,
    },
    input::{Button, ButtonController, Tri},
    number::{FixedNum, FixedWidthSignedInteger, Rect, Vector2D},
    sound::mixer::SoundChannel,
};

agb::include_gfx!("gfx/objects.toml");
agb::include_gfx!("gfx/background.toml");

extern crate agb;

type Number = FixedNum<8>;

struct Game<'a> {
    player: Player<'a>,
    input: ButtonController,
    frame_count: u32,
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

    fn update_position(&mut self) {
        self.position += self.velocity;
        self.sprite.set_position(self.position.floor());
    }

    fn collision_in_direction(&self, direction: Direction, distance: Number) -> Vector2D<Number> {
        let number_collision = Rect::new(
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
        let collider_edge = self.position.floor() + number_collision.position;
        let triple_collider: [Vector2D<i32>; 3] = match direction {
            Direction::North => [
                (0, 0).into(),
                (number_collision.size.x / 2, 0).into(),
                (number_collision.size.x, 0).into(),
            ],
            Direction::South => [
                (0, number_collision.size.y).into(),
                (number_collision.size.x / 2, number_collision.size.y).into(),
                (number_collision.size.x, number_collision.size.y).into(),
            ],
            Direction::West => [
                (0, 0).into(),
                (0, number_collision.size.y / 2).into(),
                (0, number_collision.size.y).into(),
            ],
            Direction::East => [
                (number_collision.size.x, 0).into(),
                (number_collision.size.x, number_collision.size.y / 2).into(),
                (number_collision.size.x, number_collision.size.y).into(),
            ],
        };
        for collider in triple_collider {
            let point = collider + collider_edge;
        }

        (0, 0).into()
    }

    // returns whether the entity collides going in the given direction
    fn collides_going_in_direction(&self, direction: Vector2D<Number>, distance: Number) -> bool {
        let number_collision = Rect::new(
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
        let a = self.position.floor() + number_collision.position;
        let three_colliders = [
            a + (0, number_collision.size.y).into(),
            a + (number_collision.size.x / 2, number_collision.size.y).into(),
            a + (number_collision.size.x, number_collision.size.y).into(),
        ];
        for i in three_colliders {
            let n = i + (direction * distance).floor();
            if n.y > 160 {
                return true;
            }
        }
        false
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

enum Direction {
    North,
    East,
    South,
    West,
}

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
    fn attack_frame(self, timer: u16) -> u16 {
        match self {
            SwordState::LongSword => (60 - timer) / 8,
        }
    }
    fn hold_frame(self) -> u16 {
        match self {
            SwordState::LongSword => 7,
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
        7 => 5,
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
            Rect::new((8_u16, 8_u16).into(), (4_u16, 4_u16).into()),
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

    fn update(&mut self, buttons: &ButtonController) {
        let x = buttons.x_tri();

        self.fudge_factor = (0, 0).into();

        match self.state {
            PlayerState::OnGround => match &mut self.attack_timer {
                AttackTimer::Idle => {
                    self.entity.velocity.y = 0.into();
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
                    self.state = if self
                        .entity
                        .collides_going_in_direction((0, 1).into(), 1.into())
                    {
                        PlayerState::OnGround
                    } else {
                        PlayerState::InAir
                    };

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
            },
            PlayerState::InAir => {
                let gravity: Number = 1.into();
                let gravity = gravity / 16;
                self.entity.velocity.y += gravity;
                self.state = if self
                    .entity
                    .collides_going_in_direction((0, 1).into(), 1.into())
                {
                    PlayerState::OnGround
                } else {
                    PlayerState::InAir
                };

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
            }
        }
        self.entity.velocity.x = self.entity.velocity.x * 40 / 64;

        self.entity.update_position();

        self.sprite_offset += 1;
    }

    fn commit(&mut self, offset: Vector2D<Number>) {
        self.entity.commit_with_fudge(offset, self.fudge_factor);
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
        self.player.update(&self.input);
        self.player.commit((0, 0).into());
        self.frame_count += 1;
        GameStatus::Continue
    }

    fn new(object: &ObjectControl) -> Game {
        Game {
            player: Player::new(object),
            input: ButtonController::new(),
            frame_count: 0,
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

    let mut backdrop = background.get_regular().unwrap();
    backdrop.set_position(Vector2D::new(0, 0));
    backdrop.set_map(agb::display::background::Map::new(
        tilemap::BACKGROUND_MAP,
        Vector2D::new(tilemap::WIDTH, tilemap::HEIGHT),
        0,
    ));
    backdrop.set_priority(Priority::P0);

    let mut foreground = background.get_regular().unwrap();
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

    object.enable();
    let object = object;

    let vblank = agb::interrupt::VBlank::get();
    vblank.wait_for_vblank();

    let mut game = Game::new(&object);

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
