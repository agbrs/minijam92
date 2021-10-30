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

const PURPLE_NIGHT: &[u8] = agb::include_wav!("sfx/01_-_The_Purple_Night.wav");

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

        let mut channel = SoundChannel::new(PURPLE_NIGHT);
        channel.stereo().should_loop();
        self.bgm = self.mixer.play_sound(channel);
    }
}
