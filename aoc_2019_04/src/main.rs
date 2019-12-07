fn main() {
    println!("{}", part1_brute(402328, 864247));
    println!("{}", part2(402328, 864247));
}

fn is_monotonic(password: &str) -> bool {
    let mut prev_char = '0';

    for c in password.chars() {
        if c < prev_char {
            return false;
        }
        prev_char = c;
    }

    true
}
fn has_duplicate_digit(password: &str) -> bool {
    let mut prev_char = 'a';

    for c in password.chars() {
        if c == prev_char {
            return true;
        }
        prev_char = c;
    }

    false
}

fn is_valid(password: u32) -> bool {
    let password_str: String = password.to_string();

    if password_str.chars().count() != 6 {
        false
    } else if is_monotonic(&password_str) == false {
        false
    } else if has_duplicate_digit(&password_str) == false {
        false
    } else {
        true
    }
}

fn part1_brute(min: u32, max: u32) -> u32 {
    let mut tr: u32 = 0;

    for i in min..(max + 1) {
        if is_valid(i) {
            tr = tr + 1;
        }
    }

    tr
}

enum RunningState {
    NotRunning,
    OneDup,
    BadDup
}

fn has_duplicate_digit_part2(password: &str) -> bool {
    let mut prev_char = 'a';
    let mut prev_char_running: RunningState = RunningState::NotRunning;

    for c in password.chars() {
        if c == prev_char {
            match prev_char_running {
                RunningState::NotRunning => {
                    prev_char_running = RunningState::OneDup;
                },
                _ => {
                    prev_char_running = RunningState::BadDup;
                }
            }
        } else {
            match prev_char_running {
                RunningState::OneDup => { return true; }
                _ => { prev_char_running = RunningState::NotRunning; }
            }
        }
        prev_char = c;
    }

    match prev_char_running {
        RunningState::OneDup => true,
        _ => false
    }
}

fn is_valid_part2(password: u32) -> bool {
    let password_str: String = password.to_string();

    if password_str.chars().count() != 6 {
        false
    } else if is_monotonic(&password_str) == false {
        false
    } else if has_duplicate_digit_part2(&password_str) == false {
        false
    } else {
        true
    }
}

fn part2(min: u32, max: u32) -> u32{
    let mut tr: u32 = 0;

    for i in min..(max + 1) {
        if is_valid_part2(i) {
            tr = tr + 1;
        }
    }

    tr
}
