const _BANNER_GRAPHIC_ARRAY: &[&str] = &[
    r#"  .oooooo.   oooo                      .   "#,
    r#" d8P'  `Y8b  `888                    .o8   "#,
    r#"888           888 .oo.    .oooo.   .o888oo "#,
    r#"888           888P"Y88b  `P  )88b    888   "#,
    r#"888           888   888   .oP"888    888   "#,
    r#"`88b    ooo   888   888  d8(  888    888 . "#,
    r#" `Y8bood8P'  o888o o888o `Y888""8o   "888" "#,
    r#"                                           "#,
    r#"oooooooooo.                            .   "#,
    r#"`888'   `Y8b                         .o8   "#,
    r#" 888     888   .ooooo.    .oooo.   .o888oo "#,
    r#" 888oooo888'  d88' `88b  `P  )88b    888   "#,
    r#" 888    `88b  888   888   .oP"888    888   "#,
    r#" 888    .88P  888   888  d8(  888    888 . "#,
    r#"o888bood8P'   `Y8bod8P'  `Y888""8o   "888" "#,
    r#"                                           "#,
    r#"                     __/___                "#,
    r#"               _____/______|               "#,
    r#"       _______/_____\_______\_____         "#,
    r#"       \              < < <       |        "#,
    r#"     ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~     "#,
];

const _BANNER_GRAPHIC_SERVER_ARRAY: &[&str] = &[
    r#"                                           "#,
    r#"       _____                               "#,
    r#"      / ___/___  ______   _____  _____     "#,
    r#"      \__ \/ _ \/ ___/ | / / _ \/ ___/     "#,
    r#"     ___/ /  __/ /   | |/ /  __/ /         "#,
    r#"    /____/\___/_/    |___/\___/_/          "#,
    r#"                                           "#,
];

macro_rules! _BANNER_WELCOME_MESSAGE_CLIENT {
    () => {
        r#"
Welcome to Chat Boat!
Use `help` for a list of commands.

"#
    };
}

macro_rules! _BANNER_WELCOME_MESSAGE_SERVER {
    () => {
        r#"
Welcome to Chat Boat Server!

"#
    };
}

macro_rules! _print_banner_array {
    ($comp:ident) => {
        $comp.iter().for_each(|line| println!("{}", line));
    };
}

/// Print the graphic and welcome message banner for the client app.
pub fn print_client_banner() {
    _print_banner_array!(_BANNER_GRAPHIC_ARRAY);
    print!(_BANNER_WELCOME_MESSAGE_CLIENT!());
}

/// Print the graphic and welcome message banner for the server app.
pub fn print_server_banner() {
    _print_banner_array!(_BANNER_GRAPHIC_ARRAY);
    _print_banner_array!(_BANNER_GRAPHIC_SERVER_ARRAY);
    print!(_BANNER_WELCOME_MESSAGE_SERVER!());
}
