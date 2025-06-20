pub struct AsciiArt;

impl AsciiArt {
    pub fn kernelbot_title() -> Vec<&'static str> {
        vec![
            "██╗  ██╗███████╗██████╗ ███╗   ██╗███████╗██╗     ██████╗  ██████╗ ████████╗",
            "██║ ██╔╝██╔════╝██╔══██╗████╗  ██║██╔════╝██║     ██╔══██╗██╔═══██╗╚══██╔══╝",
            "█████╔╝ █████╗  ██████╔╝██╔██╗ ██║█████╗  ██║     ██████╔╝██║   ██║   ██║   ",
            "██╔═██╗ ██╔══╝  ██╔══██╗██║╚██╗██║██╔══╝  ██║     ██╔══██╗██║   ██║   ██║   ",
            "██║  ██╗███████╗██║  ██║██║ ╚████║███████╗███████╗██████╔╝╚██████╔╝   ██║   ",
            "╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝╚═════╝  ╚═════╝    ╚═╝   ",
        ]
    }

    pub fn submit_menu_item(selected: bool) -> Vec<&'static str> {
        if selected {
            vec![
                "▶▶▶  ╔═╗╦ ╦╔╗ ╔╦╗╦╔╦╗  ◀◀◀",
                "▶▶▶  ╚═╗║ ║╠╩╗║║║║ ║   ◀◀◀",
                "▶▶▶  ╚═╝╚═╝╚═╝╩ ╩╩ ╩   ◀◀◀",
            ]
        } else {
            vec![
                "     ╔═╗╦ ╦╔╗ ╔╦╗╦╔╦╗     ",
                "     ╚═╗║ ║╠╩╗║║║║ ║      ",
                "     ╚═╝╚═╝╚═╝╩ ╩╩ ╩      ",
            ]
        }
    }

    pub fn history_menu_item(selected: bool) -> Vec<&'static str> {
        if selected {
            vec![
                "▶▶▶  ╦ ╦╦╔═╗╔╦╗╔═╗╦═╗╦ ╦  ◀◀◀",
                "▶▶▶  ╠═╣║╚═╗ ║ ║ ║╠╦╝╚╦╝  ◀◀◀",
                "▶▶▶  ╩ ╩╩╚═╝ ╩ ╚═╝╩╚═ ╩   ◀◀◀",
            ]
        } else {
            vec![
                "     ╦ ╦╦╔═╗╔╦╗╔═╗╦═╗╦ ╦     ",
                "     ╠═╣║╚═╗ ║ ║ ║╠╦╝╚╦╝     ",
                "     ╩ ╩╩╚═╝ ╩ ╚═╝╩╚═ ╩      ",
            ]
        }
    }
}

pub fn create_background_pattern(width: u16, height: u16) -> String {
    let mut pattern = String::new();
    
    for y in 0..height {
        for x in 0..width {
            // Create a pattern with dots and circuit-like characters
            let char = match (x % 8, y % 4) {
                (0, 0) => '░',
                (4, 2) => '░',
                (2, 1) => '·',
                (6, 3) => '·',
                _ => ' ',
            };
            pattern.push(char);
        }
        if y < height - 1 {
            pattern.push('\n');
        }
    }
    
    pattern
}