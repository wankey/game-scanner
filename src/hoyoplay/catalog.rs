// Official CN getGameConfigs snapshot, 2026-10-29; see docs/hoyoplay-official-sources-research.md.
// Limited to the two games with local installation evidence. Other channels are not inferred.
pub(super) struct GameConfig {
    pub biz: &'static str,
    pub name: &'static str,
    pub exe: &'static str,
}

pub(super) fn find(biz: &str) -> Option<GameConfig> {
    let (biz, name, exe) = match biz {
        "nap_cn" => ("nap_cn", "绝区零", "ZenlessZoneZero.exe"),
        "hkrpg_cn" => ("hkrpg_cn", "崩坏：星穹铁道", "StarRail.exe"),
        _ => return None,
    };
    Some(GameConfig { biz, name, exe })
}
