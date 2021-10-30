use super::rng::get_random;
use agb::number::Num;
use agb::sound::mixer::{ChannelId, Mixer, SoundChannel};

const BAT_DEATH: &[u8] = agb::include_wav!("sfx/BatDeath.wav");
const BAT_FLAP: &[u8] = agb::include_wav!("sfx/BatFlap.wav");
const JUMP1: &[u8] = agb::include_wav!("sfx/Jump1.wav");
const JUMP2: &[u8] = agb::include_wav!("sfx/Jump2.wav");
const JUMP3: &[u8] = agb::include_wav!("sfx/Jump3.wav");
const PLAYER_GETS_HIT: &[u8] = agb::include_wav!("sfx/PlayerGetsHit.wav");
const PLAYER_HEAL: &[u8] = agb::include_wav!("sfx/PlayerHeal.wav");
const PLAYER_LANDS: &[u8] = agb::include_wav!("sfx/PlayerLands.wav");
const SLIME_BOING: &[u8] = agb::include_wav!("sfx/SlimeBoing.wav");
const SLIME_DEATH: &[u8] = agb::include_wav!("sfx/SlimeDeath.wav");
const SWORD_SWING: &[u8] = agb::include_wav!("sfx/SwordSwing.wav");

const PURPLE_NIGHT: &[u8] = agb::include_wav!("sfx/01 - The Purple Night (Main Loop).wav");
const SUNRISE: &[u8] = agb::include_wav!("sfx/02 - Sunrise (Main Loop).wav");

pub struct Sfx<'a> {
    bgm: Option<ChannelId>,
    mixer: &'a mut Mixer,
}

impl<'a> Sfx<'a> {
    pub fn new(mixer: &'a mut Mixer) -> Self {
        Self { mixer, bgm: None }
    }

    pub fn vblank(&mut self) {
        self.mixer.vblank();
    }

    pub fn purple_night(&mut self) {
        if let Some(bgm) = &self.bgm {
            let channel = self.mixer.get_channel(&bgm).unwrap();
            channel.stop();
        }

        let mut channel = SoundChannel::new_high_priority(PURPLE_NIGHT);
        channel.stereo().should_loop();
        self.bgm = self.mixer.play_sound(channel);
    }

    pub fn jump(&mut self) {
        let r = get_random() % 3;

        let channel = match r {
            0 => SoundChannel::new(JUMP1),
            1 => SoundChannel::new(JUMP2),
            _ => SoundChannel::new(JUMP3),
        };

        self.mixer.play_sound(channel);
    }

    pub fn sword(&mut self) {
        self.mixer.play_sound(SoundChannel::new(SWORD_SWING));
    }

    pub fn slime_boing(&mut self) {
        let mut channel = SoundChannel::new(SLIME_BOING);
        let one: Num<i16, 4> = 1.into();
        channel.volume(one / 4);
        self.mixer.play_sound(channel);
    }

    pub fn slime_dead(&mut self) {
        let channel = SoundChannel::new(SLIME_DEATH);
        self.mixer.play_sound(channel);
    }

    pub fn player_hurt(&mut self) {
        self.mixer.play_sound(SoundChannel::new(PLAYER_GETS_HIT));
    }

    pub fn player_heal(&mut self) {
        self.mixer.play_sound(SoundChannel::new(PLAYER_HEAL));
    }

    pub fn player_land(&mut self) {
        self.mixer.play_sound(SoundChannel::new(PLAYER_LANDS));
    }

    pub fn bat_flap(&mut self) {
        self.mixer.play_sound(SoundChannel::new(BAT_FLAP));
    }

    pub fn bat_death(&mut self) {
        self.mixer.play_sound(SoundChannel::new(BAT_DEATH));
    }
}
