use unicode_linebreak::linebreaks;

/// given a line, return any linebreaks
pub fn get_linebreak_locations(line: &str, width: usize) -> Vec<usize> {
    let mut breaks = vec![];
    let row = 0;
    let mut last_opp = None;
    for (i, _opp) in linebreaks(line.trim_end()) {
        if row + i >= width {
            if let Some(br) = last_opp {
                breaks.push(br);
            }
        }
        last_opp = Some(i);
    }
    breaks
}

#[test]
fn first_lb() {
    let line = "12345 67890
1234567 890";
    println!("lbr {:?}", linebreaks(line).collect::<Vec<(usize, unicode_linebreak::BreakOpportunity)>>());

    println!("lb {:?}", get_linebreak_locations(line, 3));
    //assert!(get_linebreak_locations(line, 5) == vec![5]);
}
