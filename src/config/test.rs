use super::*;

#[test]
fn upgrade_increase() {
    for arch in CONFIG.plant_archetypes.iter() {
        let adv = &arch.advancements;
        let last = adv.rest.last().unwrap();
        for xp in 0..last.xp {
            assert!(
                adv.current(xp).xp <= xp,
                "when xp is {} for {} the current advancement has more xp({})",
                xp,
                arch.name,
                adv.current(xp).xp
            );
        }
    }
}

#[test]
fn archetype_name_matches() {
    check_archetype_name_matches(&*CONFIG).unwrap_or_else(|e| panic!("{}", e));
}
