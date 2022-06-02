pub fn duration_text(dur: i64) -> String {
    let min = dur / 60;
    let secs = dur % 60;

    format!("{}:{:02}", min, secs)
}