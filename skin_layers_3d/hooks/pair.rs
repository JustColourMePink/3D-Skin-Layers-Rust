// Options > Mods: a native mod's buttons whose labels start with U+0001 are half
// width, two to a row (as the Key Binds rows are), the way the original's
// settings screen sets its six part switches side by side.
fn pair(value: Vec<Row>, s: &Settings, i: usize) -> Vec<Row> {
    const HALF: char = '\u{1}';
    let Some(page) = s.mods.installed.get(i).and_then(|m| m.native.as_ref()) else {
        return value;
    };
    if !page.buttons.iter().any(|l| l.starts_with(HALF)) {
        return value;
    }
    // The page's buttons are its last rows, one each: lay them out again.
    let keep = value.len().saturating_sub(page.buttons.len());
    let mut rows: Vec<Row> = value.into_iter().take(keep).collect();
    let widget = |b: usize, width: f32| {
        let label = page.buttons[b].trim_start_matches(HALF).to_owned();
        button(0.0, 0.0, width, Text::literal(label), Act::NativeButton(i, b))
    };
    let half = |b: usize| page.buttons.get(b).is_some_and(|l| l.starts_with(HALF));
    let mut b = 0;
    while b < page.buttons.len() {
        if half(b) && half(b + 1) {
            rows.push(Row::Buttons(widget(b, 150.0), Some(widget(b + 1, 150.0))));
            b += 2;
        } else {
            rows.push(Row::Buttons(widget(b, 310.0), None));
            b += 1;
        }
    }
    rows
}
