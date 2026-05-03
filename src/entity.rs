#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MonsterKind {
    Goblin = 0,
    Mage = 1,
    Rat = 2,
    Lich = 3,
}

impl From<u8> for MonsterKind {
    fn from(b: u8) -> Self {
        match b {
            1 => MonsterKind::Mage,
            2 => MonsterKind::Rat,
            3 => MonsterKind::Lich,
            _ => MonsterKind::Goblin,
        }
    }
}

impl MonsterKind {
    pub fn char(&self) -> char {
        match self {
            MonsterKind::Goblin => 'G',
            MonsterKind::Mage => 'M',
            MonsterKind::Rat => 'R',
            MonsterKind::Lich => 'L',
        }
    }

    pub fn starting_hp(&self) -> i32 {
        match self {
            MonsterKind::Goblin => 8,
            MonsterKind::Mage => 12,
            MonsterKind::Rat => 5,
            MonsterKind::Lich => 40,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MonsterKind::Goblin => "Goblin",
            MonsterKind::Mage => "Mage",
            MonsterKind::Rat => "Rat",
            MonsterKind::Lich => "Lich",
        }
    }
}

pub struct Player {
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub max_hp: i32,
}

impl Player {
    pub fn new(x: usize, y: usize) -> Self {
        Player {
            x,
            y,
            hp: 100,
            max_hp: 100,
        }
    }
}

pub struct Monster {
    pub x: usize,
    pub y: usize,
    pub kind: MonsterKind,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub teleported: bool,
}

impl Monster {
    pub fn new(x: usize, y: usize, kind: MonsterKind) -> Self {
        let hp = kind.starting_hp();
        Monster {
            x,
            y,
            kind,
            hp,
            max_hp: hp,
            alive: true,
            teleported: false,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive && self.hp > 0
    }
}
