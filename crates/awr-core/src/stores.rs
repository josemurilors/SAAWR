//! What the game's own `shopping.dat` says about clothes, hair and the shops that sell them.
//! It is only read: a new item never goes into that file (the game shows 12 items per body
//! part, holds 560 prices and ModLoader merges the file by line position). The CLEO catalog
//! (see `catalog.rs`) sells the items instead; this module gives it the shop list and gives
//! the app its suggestions (body part, model, price and shops from the original texture).

/// Body part of hair: sold by the barbers, from the `Haircuts` prices.
pub const HAIR: u8 = 1;
/// Parts of the body that can be sold: torso, legs, shoes, necklace, watch, glasses, hat, hair.
pub const BODY_PARTS: [u8; 8] = [0, 2, 3, 13, 14, 15, 16, HAIR];

/// A shop the catalog script knows. `area`, `x` and `y` place it in the interior space: the
/// script compares the player's interior and position (sources: MTA wiki "Interior IDs").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shop {
    /// Section in `shopping.dat` (`CSchp`, `barbers`, ...).
    pub section: &'static str,
    pub label: &'static str,
    /// `clothes` or `haircuts`.
    pub kind: &'static str,
    pub area: i32,
    pub x: f32,
    pub y: f32,
}

const fn shop(
    section: &'static str,
    label: &'static str,
    kind: &'static str,
    area: i32,
    x: f32,
    y: f32,
) -> Shop {
    Shop {
        section,
        label,
        kind,
        area,
        x,
        y,
    }
}

/// Index = the shop number the script uses (`[Shop<n>]`, `[List<n>]`, `AWRS<n>`).
pub const SHOPS: [Shop; 9] = [
    shop("CSchp", "Binco", "clothes", 15, 207.543, -109.004),
    shop("CSsprt", "ProLaps", "clothes", 3, 207.356, -138.0029),
    shop("LACS1", "Sub Urban", "clothes", 1, 203.8173, -46.5385),
    shop("clothgp", "Zip", "clothes", 18, 161.462, -91.394),
    shop("Csdesgn", "Victim", "clothes", 5, 221.331, -6.6169),
    shop("Csexl", "Didier Sachs", "clothes", 14, 204.1789, -165.874),
    shop(
        "barbers",
        "Barbearia Reece's",
        "haircuts",
        2,
        411.6259,
        -21.4332,
    ),
    shop(
        "barber2",
        "Barbearia Gay Gordo's",
        "haircuts",
        3,
        418.653,
        -82.639,
    ),
    shop(
        "barber3",
        "Barbearia Macisla's",
        "haircuts",
        12,
        411.641,
        -51.846,
    ),
];

/// The shop section is one of ours and sells what `part` is.
pub fn shop_sells(section: &str, part: u8) -> bool {
    SHOPS
        .iter()
        .find(|s| s.section.eq_ignore_ascii_case(section))
        .is_some_and(|s| s.kind == if part == HAIR { "haircuts" } else { "clothes" })
}

/// An item of the game's own clothes or haircuts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Original {
    /// Texture name (lower case): the item's key in `shopping.dat`.
    pub texture: String,
    /// Model it is worn on (lower case).
    pub model: String,
    pub part: u8,
    pub price: u32,
    /// Sections of the shops that sell it.
    pub shops: Vec<String>,
}

fn words(line: &str) -> Vec<&str> {
    let line = line.split('#').next().unwrap_or_default();
    line.split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect()
}

/// Every clothing and haircut price line, with the shops that list it.
pub fn originals(text: &str) -> Vec<Original> {
    let mut parents: Vec<String> = vec![];
    let mut out: Vec<Original> = vec![];
    let mut sold: Vec<(String, &'static str)> = vec![]; // (texture, our shop section)
    for line in text.lines() {
        let w = words(line);
        match w[..] {
            [] => {}
            ["section", name, ..] => parents.push(name.to_ascii_lowercase()),
            ["end", ..] => {
                parents.pop();
            }
            _ => match parents.iter().map(String::as_str).collect::<Vec<_>>()[..] {
                ["prices", "clothes" | "haircuts"] => {
                    if let (Some(&tex), Some(&model), Some(part), Some(price)) = (
                        w.first(),
                        w.get(2),
                        w.get(3).and_then(|p| p.parse::<u8>().ok()),
                        w.last().and_then(|p| p.parse::<u32>().ok()),
                    ) {
                        out.push(Original {
                            texture: tex.to_lowercase(),
                            model: model.to_lowercase(),
                            part,
                            price,
                            shops: vec![],
                        });
                    }
                }
                ["shops", section] if w[0].eq_ignore_ascii_case("item") && w.len() > 1 => {
                    if let Some(s) = SHOPS
                        .iter()
                        .find(|s| s.section.eq_ignore_ascii_case(section))
                    {
                        sold.push((w[1].to_lowercase(), s.section));
                    }
                }
                _ => {}
            },
        }
    }
    for o in &mut out {
        o.shops = sold
            .iter()
            .filter(|(t, _)| *t == o.texture)
            .map(|(_, s)| s.to_string())
            .collect();
    }
    out
}

/// The game's own `data/shopping.dat`, parsed (empty if it cannot be read).
pub fn load(data: &std::path::Path) -> Vec<Original> {
    std::fs::read(data.join("shopping.dat")).map_or(vec![], |b| {
        originals(&b.iter().map(|&c| char::from(c)).collect::<String>())
    })
}

/// `(model, body part)` of the original models, sorted, to wear a new texture on.
pub fn models(db: &[Original]) -> Vec<(String, u8)> {
    let mut m: Vec<(String, u8)> = db.iter().map(|o| (o.model.clone(), o.part)).collect();
    m.sort();
    m.dedup();
    m
}

/// What the app suggests for a new item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub part: u8,
    /// Original model to wear the texture on.
    pub model: String,
    pub price: u32,
    /// Shop sections that sold the original item.
    pub shops: Vec<String>,
    /// The original texture (or model) the suggestion comes from.
    pub from: String,
}

/// The new item is a texture of the same name as an original one (what packs that replace
/// clothes do), or else a model of the same name as an original model.
pub fn suggest(db: &[Original], texture: &str, dff: Option<&str>) -> Option<Suggestion> {
    let texture = texture.to_lowercase();
    let by_texture = db.iter().find(|o| o.texture == texture);
    let by_model = || {
        let m = dff?.to_lowercase();
        db.iter().find(|o| o.model == m)
    };
    let o = by_texture.or_else(by_model)?;
    Some(Suggestion {
        part: o.part,
        model: o.model.clone(),
        price: o.price,
        shops: o.shops.clone(),
        from: if by_texture.is_some() {
            o.texture.clone()
        } else {
            o.model.clone()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = "section prices\r\n\tsection CarMods\r\n\t\tbnt_b_sc_l\t\tBMBLBS\t\trespect 0 \tsexy 0\t\t150\r\n\tend\r\n\tsection Clothes\r\n\t\t# texture nametag model type\r\n\t\tvest\t\tVESTWH\t\tvest\t\t0\t\trespect 2 \tsexy 3\t\t10\r\n\t\tlegsblack\t\tLEGSBL\t\tlegs\t\t2\t\trespect 0 \tsexy 0\t\t20 # END\r\n\tend\r\n\tsection Haircuts\r\n\t\tcornrows\t\tCORNROW\t\tcornrows\t\t1\t\trespect 30 \tsexy 30\t\t500\r\n\tend\r\nend\r\nsection shops\r\n\tsection CSchp # BINCO\r\n\t\ttype clothes\r\n\t\titem vest\r\n\t\titem legsblack\r\n\tend\r\n\tsection Csexl\r\n\t\ttype clothes\r\n\t\titem vest\r\n\tend\r\n\tsection uniform\r\n\t\ttype clothes\r\n\t\titem legsblack\r\n\tend\r\n\tsection barbers\r\n\t\ttype haircuts\r\n\t\titem cornrows\r\n\tend\r\nend\r\n";

    #[test]
    fn reads_clothes_haircuts_and_the_shops_that_sell_them() {
        let db = originals(DAT);
        assert_eq!(db.len(), 3);
        let vest = &db[0];
        assert_eq!(
            (
                vest.texture.as_str(),
                vest.model.as_str(),
                vest.part,
                vest.price
            ),
            ("vest", "vest", 0, 10)
        );
        assert_eq!(vest.shops, ["CSchp", "Csexl"]);
        assert_eq!(db[1].shops, ["CSchp"], "the `uniform` list is not a shop");
        assert_eq!(
            (db[2].part, db[2].price, db[2].shops.clone()),
            (HAIR, 500, vec!["barbers".to_string()])
        );
    }

    #[test]
    fn suggests_from_the_original_texture_then_the_model() {
        let db = originals(DAT);
        let s = suggest(&db, "LEGSBLACK", None).unwrap();
        assert_eq!(
            (s.part, s.model.as_str(), s.price, s.from.as_str()),
            (2, "legs", 20, "legsblack")
        );
        let s = suggest(&db, "mynewvest", Some("vest")).unwrap();
        assert_eq!((s.part, s.from.as_str(), s.shops.len()), (0, "vest", 2));
        assert!(suggest(&db, "landstal", Some("landstal")).is_none());
        assert_eq!(
            models(&db),
            [
                ("cornrows".to_string(), 1),
                ("legs".into(), 2),
                ("vest".into(), 0)
            ]
        );
    }

    #[test]
    fn shops_sell_what_their_kind_says() {
        assert!(shop_sells("csexl", 0) && shop_sells("barbers", HAIR));
        assert!(!shop_sells("barbers", 0) && !shop_sells("CSchp", HAIR) && !shop_sells("nope", 0));
        // the script finds the shop by interior and position: ProLaps and a barber share area 3
        let (a, b) = (SHOPS[1], SHOPS[7]);
        assert_eq!(a.area, b.area);
        assert!((a.x - b.x).hypot(a.y - b.y) > 140.0);
    }

    #[test]
    fn real_game_file_gives_suggestions_for_the_test_packs() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let f = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/shopping.dat"
        ))
        .unwrap();
        let db = originals(&f.iter().map(|&b| char::from(b)).collect::<String>());
        let s = suggest(&db, "watchyellow", None).unwrap();
        assert_eq!(
            (s.part, s.model.as_str(), s.price, s.shops.clone()),
            (14, "watch", 15, vec!["CSchp".to_string()])
        );
        let s = suggest(&db, "cornrowsb", Some("cornrows")).unwrap();
        assert_eq!(
            (s.part, s.shops.clone()),
            (HAIR, vec!["barber2".to_string()])
        );
        assert!(models(&db)
            .iter()
            .any(|(m, p)| m == "bikerhelmet" && *p == 16));
    }
}
