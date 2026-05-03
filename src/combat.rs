use crate::entity::{Monster, MonsterKind, Player};
use rand::Rng;

pub fn roll_d6(rng: &mut impl Rng) -> i32 {
    rng.gen_range(1..=6)
}

pub fn resolve_player_attack(
    player: &mut Player,
    monster: &mut Monster,
    rng: &mut impl Rng,
) -> String {
    if !monster.alive {
        return format!("The {} is already dead.", monster.kind.name());
    }
    let hit_roll = roll_d6(rng);
    if hit_roll < 4 {
        return format!("You missed the {}!", monster.kind.name());
    }

    let damage = roll_d6(rng);
    monster.hp -= damage;

    if monster.hp <= 0 {
        monster.alive = false;
        player.max_hp += 1;
        player.hp = (player.hp + 1).min(player.max_hp);
        return format!(
            "You slew the {}! +1 max HP (now {}/{})",
            monster.kind.name(),
            player.hp,
            player.max_hp
        );
    }

    format!(
        "You hit the {} for {} damage! ({}/{} HP)",
        monster.kind.name(),
        damage,
        monster.hp.max(0),
        monster.max_hp,
    )
}

pub fn resolve_monster_attack(
    monster: &Monster,
    player: &mut Player,
    rng: &mut impl Rng,
    multiplayer: bool,
) -> (String, bool) {
    let hit_roll = roll_d6(rng);
    if hit_roll < 4 {
        return (format!("The {} missed you!", monster.kind.name()), false);
    }

    let damage = match monster.kind {
        MonsterKind::Lich => roll_d6(rng) + roll_d6(rng),
        _ => if multiplayer { roll_d6(rng).max(roll_d6(rng)) } else { roll_d6(rng) },
    };
    player.hp -= damage;

    let died = player.hp <= 0;
    let msg = format!(
        "The {} hits you for {} damage! ({}/{} HP)",
        monster.kind.name(),
        damage,
        player.hp.max(0),
        player.max_hp,
    );
    (msg, died)
}
