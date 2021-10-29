#![no_std]
#![no_main]

use agb::{
    display::{
        object::{ObjectControl, ObjectStandard},
        Priority,
    },
    input::{Button, ButtonController, Tri},
    number::{FixedNum, FixedWidthSignedInteger, Rect, Vector2D},
    sound::mixer::SoundChannel,
};

agb::include_gfx!("gfx/objects.toml");

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
}

enum PlayerState {
    OnGround,
    Rising,
    Falling,
}

enum SwordState {
    LongSword,
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
        entity.sprite.set_position((10, 10).into());
        entity.sprite.commit();

        Player {
            entity,
            facing: Tri::Zero,
            state: PlayerState::OnGround,
            sword: SwordState::LongSword,
            sprite_offset: 0,
            attack_timer: AttackTimer::Idle,
        }
    }

    fn update(&mut self, buttons: &ButtonController) {
        let x = buttons.x_tri();

        let mut position_fudge_factor: Vector2D<i32> = (0, 0).into();

        match self.state {
            PlayerState::OnGround => match &mut self.attack_timer {
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

                    if buttons.is_just_pressed(Button::A) {
                        self.attack_timer = AttackTimer::Attack(60);
                    }
                }
                AttackTimer::Attack(a) => {
                    *a -= 1;
                    let sprite_id = (60 - *a) / 8;
                    let x_fudge = match sprite_id {
                        0 => 0,
                        1 => 0,
                        2 => -1,
                        3 => -4,
                        4 => -5,
                        5 => -5,
                        6 => -5,
                        7 => -5,
                        _ => unreachable!(),
                    };
                    position_fudge_factor.x = x_fudge * self.facing as i32;
                    self.entity.sprite.set_tile_id((16 + sprite_id) * 4);
                    if *a == 0 {
                        self.attack_timer = AttackTimer::Cooldown(20);
                    }
                }
                AttackTimer::Cooldown(a) => {
                    *a -= 1;
                    position_fudge_factor.x = -5 * self.facing as i32;
                    if *a == 0 {
                        self.attack_timer = AttackTimer::Idle;
                    }
                }
            },
            PlayerState::Falling => {}
            PlayerState::Rising => {}
        }
        self.entity.velocity.x = self.entity.velocity.x * 40 / 64;

        self.entity.update_position();
        self.entity
            .sprite
            .set_position(self.entity.position.floor() - position_fudge_factor);

        self.sprite_offset += 1;
    }

    fn commit(&self) {
        self.entity.sprite.commit();
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
        self.player.commit();
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

const MINIMUSIC: &[u8] = agb::include_wav!("sfx/Mini_Jam_92.wav");

fn game_with_level(gba: &mut agb::Gba) {
    let mut object = gba.display.object.get();
    object.set_sprite_palettes(objects::objects.palettes);
    object.set_sprite_tilemap(objects::objects.tiles);

    let mut background = gba.display.video.tiled0();
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

#[agb::entry]
fn main() -> ! {
    let mut gba = agb::Gba::new();

    game_with_level(&mut gba);

    loop {}
}
