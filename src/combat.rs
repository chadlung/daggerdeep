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
    level: u8,
) -> (String, bool) {
    let hit_roll = roll_d6(rng);
    if hit_roll < 4 {
        return (format!("The {} missed you!", monster.kind.name()), false);
    }

    let mut damage = match monster.kind {
        MonsterKind::Lich => roll_d6(rng) + roll_d6(rng),
        _ => if multiplayer { roll_d6(rng).max(roll_d6(rng)) } else { roll_d6(rng) },
    };
    // The deep dungeon hits harder: every landed hit gains +2 from level 7 down.
    // The Lich is exempt and always deals its original 2d6.
    if level >= 7 && monster.kind != MonsterKind::Lich {
        damage += 2;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Damage one attack from `kind` deals to a fresh player at `level`, or 0 if
    /// it missed. The seed fixes the roll sequence so runs are comparable across levels.
    fn damage_at(kind: MonsterKind, level: u8, seed: u64, multiplayer: bool) -> i32 {
        let monster = Monster::new(0, 0, kind);
        let mut player = Player::new(0, 0);
        let mut rng = StdRng::seed_from_u64(seed);
        resolve_monster_attack(&monster, &mut player, &mut rng, multiplayer, level);
        player.max_hp - player.hp
    }

    #[test]
    fn levels_7_and_deeper_add_two_to_every_landed_hit() {
        for seed in 0..100 {
            let shallow = damage_at(MonsterKind::Goblin, 6, seed, false);
            for level in [7u8, 8, 9, 10] {
                let deep = damage_at(MonsterKind::Goblin, level, seed, false);
                if shallow == 0 {
                    assert_eq!(deep, 0, "a miss must stay a miss (level {level}, seed {seed})");
                } else {
                    assert_eq!(deep, shallow + 2, "level {level}, seed {seed}");
                }
            }
        }
    }

    #[test]
    fn the_lich_keeps_its_original_damage_at_every_depth() {
        for seed in 0..100 {
            let baseline = damage_at(MonsterKind::Lich, 6, seed, false);
            for level in [7u8, 8, 9, 10] {
                assert_eq!(
                    damage_at(MonsterKind::Lich, level, seed, false),
                    baseline,
                    "the Lich must not gain the deep-level bonus (level {level}, seed {seed})"
                );
            }
        }
    }

    #[test]
    fn levels_below_7_are_unaffected() {
        for seed in 0..100 {
            let baseline = damage_at(MonsterKind::Goblin, 1, seed, false);
            for level in 2u8..=6 {
                let dmg = damage_at(MonsterKind::Goblin, level, seed, false);
                assert_eq!(dmg, baseline, "level {level}, seed {seed}");
            }
        }
    }

    #[test]
    fn the_bonus_applies_in_multiplayer_too() {
        for seed in 0..100 {
            let shallow = damage_at(MonsterKind::Goblin, 6, seed, true);
            let deep = damage_at(MonsterKind::Goblin, 7, seed, true);
            if shallow == 0 {
                assert_eq!(deep, 0, "seed {seed}");
            } else {
                assert_eq!(deep, shallow + 2, "seed {seed}");
            }
        }
    }

    #[test]
    fn a_deep_level_hit_is_never_below_the_boosted_minimum() {
        // Solo, non-Lich: either a miss (0) or 1d6 + 2, so 3 through 8.
        for seed in 0..300 {
            let dmg = damage_at(MonsterKind::Goblin, 10, seed, false);
            assert!(dmg == 0 || (3..=8).contains(&dmg), "damage {dmg} (seed {seed})");
        }
    }

    #[test]
    fn a_deep_lich_hit_stays_in_its_unboosted_2d6_band() {
        // Either a miss (0) or a plain 2d6, so 2 through 12 -- never 14.
        for seed in 0..300 {
            let dmg = damage_at(MonsterKind::Lich, 10, seed, false);
            assert!(dmg == 0 || (2..=12).contains(&dmg), "damage {dmg} (seed {seed})");
        }
    }
}
