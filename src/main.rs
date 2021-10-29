#![no_std]
#![no_main]

use agb::{
    display::{
        object::{ObjectControl, ObjectStandard},
        Priority,
    },
    input::{ButtonController, Tri},
    number::{FixedNum, Rect, Vector2D},
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

struct Player<'a> {
    entity: Entity<'a>,
    facing: Tri,
    state: PlayerState,
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
        }
    }

    fn update(&mut self, buttons: &ButtonController) {
        let x = buttons.x_tri();
        if x != Tri::Zero {
            self.facing = x;
        }

        match self.state {
            PlayerState::OnGround => {
                self.entity.sprite.set_hflip(self.facing == Tri::Negative);
                self.entity.velocity.x += Number::new(x as i32) / 8;
                self.entity.velocity.x = self.entity.velocity.x * 54 / 64;
            }
            PlayerState::Falling => {}
            PlayerState::Rising => {}
        }
        self.entity.update_position();
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
