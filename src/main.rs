#![no_std]
#![no_main]

use agb::{
    display::{
        object::{ObjectControl, ObjectStandard},
        Priority,
    },
    input::ButtonController,
    number::{FixedNum, Rect, Vector2D},
};

extern crate agb;

type Number = FixedNum<8>;

struct Game<'a> {
    player: Player<'a>,
    input: ButtonController,
}

struct Player<'a> {
    entity: Entity<'a>,
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
}

impl<'a> Player<'a> {
    fn new(object_controller: &'a ObjectControl) -> Player {
        Player {
            entity: Entity::new(
                object_controller,
                Rect::new((8_u16, 8_u16).into(), (4_u16, 4_u16).into()),
            ),
        }
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
        GameStatus::Continue
    }

    fn new(object: &ObjectControl) -> Game {
        Game {
            player: Player::new(object),
            input: ButtonController::new(),
        }
    }
}

fn game_with_level(gba: &mut agb::Gba) {
    let mut object = gba.display.object.get();
    object.enable();
    let object = object;

    let mut background = gba.display.video.tiled0();

    let vblank = agb::interrupt::VBlank::get();
    vblank.wait_for_vblank();

    let mut game = Game::new(&object);

    loop {
        game.advance_frame();
        vblank.wait_for_vblank();
    }
}

#[agb::entry]
fn main() -> ! {
    let mut gba = agb::Gba::new();

    game_with_level(&mut gba);

    loop {}
}
