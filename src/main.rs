use clap::{arg, command, Command};
use markdown::{mdast, ParseOptions};
use regex::Regex;

use std::{
    collections::{hash_map::Entry, HashMap},
    fs,
    io::{self, Write},
    iter,
    path::PathBuf,
    time::{Duration, SystemTime},
};

fn main() {
    let command = cli();
    let config = Config::from(&command);
    let matches = command.get_matches();

    if matches.subcommand().is_some_and(|(cmd, _)| cmd == "watch") {
        watch_mode(&config);
    } else {
        compile_all(&config);
    }
}

fn cli() -> Command {
    command!()
        .about("Generate static websites from a directory of markdown files.")
        .subcommand(
            Command::new("watch")
                .short_flag('w')
                .about("Watch for changes and automatically recompile."),
        )
        .arg(arg!(-i --input [DIRECTORY] "The project root of the markdown files."))
        .arg(arg!(-o --output [DIRECTORY] "The destination for the compiled webpage."))
        .arg(arg!(--style [FILE] "The CSS-stylesheet to use for all html files."))
        .arg(arg!(--header [FILE] "The HTML-header to prepend to all HTML-Bodies."))
        .arg(arg!(--footer [FILE] "The HTML-footer to append to all HTML-Bodies."))
}

#[derive(Debug)]
struct Config {
    input_dir: PathBuf,
    output_dir: PathBuf,
    stylesheet: Option<PathBuf>,
    header: Option<String>,
    footer: Option<String>,
}

impl Config {
    pub fn from(command: &Command) -> Self {
        let matches = command.clone().get_matches();

        let input_dir = match matches.get_one::<String>("input") {
            Some(path) => PathBuf::from(path),
            _ => PathBuf::from("src"),
        };

        if !input_dir.is_dir() {
            eprintln!("error: specified input directory does not exist or is not a directory.\n\nUsage: mdmake [OPTIONS] [COMMAND]\n\nFor more information, try '--help'.");
        }

        let output_dir = match matches.get_one::<String>("output") {
            Some(path) => PathBuf::from(path),
            _ => PathBuf::from("out"),
        };

        if output_dir.is_file() {
            eprintln!("error: specified output directory path is an existing file.\n\nUsage: mdmake [OPTIONS] [COMMAND]\n\nFor more information, try '--help'.");
        }

        Config {
            input_dir: input_dir.clone(),
            output_dir,
            stylesheet: match matches.get_one::<String>("style") {
                Some(path) => Some(PathBuf::from(path)),
                None => {
                    let path = input_dir.join("style.css");
                    fs::metadata(&path).is_ok().then_some(path)
                }
            },
            header: match matches.get_one::<String>("header") {
                Some(path) => String::get_file_contents(path),
                _ => {
                    let file = input_dir.join("header.html");
                    match fs::metadata(&file) {
                        Ok(_) => file.get_file_contents(),
                        _ => None,
                    }
                }
            },
            footer: match matches.get_one::<String>("header") {
                Some(path) => String::get_file_contents(path),
                _ => {
                    let file = input_dir.join("footer.html");
                    match fs::metadata(&file) {
                        Ok(_) => file.get_file_contents(),
                        _ => None,
                    }
                }
            },
        }
    }
}

fn watch_mode(config: &Config) {
    compile_all(config);
    copy_stylesheet_to_output_dir(config);

    let mut last_modified_times = HashMap::new();

    if let Ok(paths) = walk_dir(&config.input_dir) {
        for path in paths {
            let time = fs::metadata(&path)
                .and_then(|data| data.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            last_modified_times.insert(path, time);
        }
    }

    loop {
        let walk_result = walk_dir(&config.input_dir);
        if walk_result.is_err() {
            continue;
        }

        let paths = walk_result.unwrap();
        for path in paths {
            if let Ok(metadata) = fs::metadata(&path) {
                let modified_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

                match last_modified_times.entry(path.clone()) {
                    Entry::Occupied(entry) => {
                        if modified_time > *entry.get() {
                            println!("File has been modified: {}!", path.to_str().unwrap());
                            println!("Recompiling...");
                            compile_file(path.clone(), config);
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(modified_time);
                        println!("New File has been added: {}!", path.to_str().unwrap());
                        println!("Compiling...");
                        compile_file(path.clone(), config);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn compile_all(config: &Config) {
    let _ = fs::remove_dir_all(&config.output_dir);
    let _ = fs::create_dir_all(&config.output_dir);

    copy_stylesheet_to_output_dir(config);

    fn compile_all_recurse(subdir: &PathBuf, config: &Config) {
        for entry in fs::read_dir(subdir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                compile_all_recurse(&path, config);
            } else {
                let output_relative_path = config
                    .output_dir
                    .join(path.strip_prefix(&config.input_dir).unwrap());

                if let Some(parent) = output_relative_path.parent() {
                    fs::create_dir_all(parent)
                        .expect("Failed to create necessary subdirectory in output directory.");
                }

                if path.extension().unwrap_or_default().to_ascii_lowercase() == "md" {
                    compile_file(path, config);
                } else {
                    fs::copy(path, output_relative_path)
                        .expect("Failed to copy resource file to output directory.");
                }
            }
        }
    }

    compile_all_recurse(&config.input_dir, config)
}

fn compile_file(input_file: PathBuf, config: &Config) {
    let output_file_relative = input_file
        .strip_prefix(&config.input_dir)
        .expect("File that was tried to compile seems to not be within the input directory.")
        .with_extension("html");

    let output_file = config.output_dir.join(output_file_relative.clone());
    fs::File::create(&output_file).expect("Failed to clear/create output HTML-file.");

    let mut out_fd = fs::OpenOptions::new()
        .append(true)
        .open(&output_file)
        .expect("Couldn't open output HTML-file for appending.");

    writeln!(out_fd, "<head>").expect("Failed writing HTML <head> opening tag to output file.");

    let ast = markdown::to_mdast(
        &fs::read_to_string(&input_file).expect("Failed to read input MD file to String."),
        &ParseOptions::default(),
    )
    .unwrap();

    if let Some(title_tag) = get_file_title_html_tag(ast) {
        write!(out_fd, "{}", title_tag).expect("Failed writing HTML <title> tag to output file.");
    }

    if let Some(style_link_tag) = get_html_style_link_tag(config, output_file_relative) {
        write!(out_fd, "{}", style_link_tag)
            .expect(r#"Failed writing HTML <link rel="stylesheet"> tag to output file."#);
    }

    writeln!(out_fd, "\n</head>\n<body>")
        .expect("Failed writing </head> closing tag and <body> opening tag to HTML output file.");

    if let Some(header) = &config.header {
        write!(out_fd, "{}", header)
            .expect("Failed writing header file contents to output HTML file.");
    }

    let markdown = fs::read_to_string(input_file).expect("Failed to read input MD file to String.");

    let md_contents = replace_md_link_extensions_with_html(&markdown);
    let main_body = markdown::to_html(&md_contents);
    write!(out_fd, "{}", main_body).expect("Couldn't append HTML-body to output HTML-file.");

    if let Some(footer) = &config.footer {
        write!(out_fd, "{}", footer)
            .expect("Failed writing footer file contents to output HTML file.");
    }
    write!(out_fd, "</body>").expect("Failed writing HTML </body> closing tag to output file.");
}

fn walk_dir(start_dir: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut entries = Vec::new();

    if start_dir.is_dir() {
        for entry in fs::read_dir(start_dir)? {
            let entry = entry?.path();

            if entry.is_dir() {
                entries.extend(walk_dir(&entry)?);
            } else {
                entries.push(entry);
            }
        }
    }
    Ok(entries)
}

fn replace_md_link_extensions_with_html(markdown: &str) -> String {
    Regex::new(r"\[([^)]*)\]\(([^)]*)\)")
        .unwrap()
        .replace_all(markdown, |caps: &regex::Captures| {
            let link_text = &caps[1];
            let link_url = &caps[2];
            let mut path = PathBuf::from(link_url);
            if path.is_relative()
                && path.extension().map(|ex| ex.to_ascii_lowercase()) == Some("md".into())
            {
                path = path.with_extension("html");
            }
            format!(
                "[{}]({})",
                link_text,
                path.to_str().expect("Failed to convert PathBuf to String.")
            )
        })
        .to_string()
}

fn copy_stylesheet_to_output_dir(config: &Config) {
    if let Some(stylesheet) = &config.stylesheet {
        let exists = fs::metadata(stylesheet).is_ok();

        if exists {
            fs::copy(stylesheet, config.output_dir.join("style.css"))
                .expect("Failed to copy CSS-stylesheet to root of output directory.");
        }
    }
}

fn get_file_title_html_tag(ast: mdast::Node) -> Option<String> {
    let mut page_title = None;
    let children = ast.children();

    if let Some(children) = children {
        for child in children {
            if let mdast::Node::Heading(h) = child {
                if h.depth == 1 {
                    page_title = Some(h.children.get_string_contents());
                    break;
                }
            }
        }
    }

    page_title
        .is_some()
        .then(|| format!("<title>{}</title>\n", page_title.unwrap()))
}

fn get_html_style_link_tag(config: &Config, output_file_relative: PathBuf) -> Option<String> {
    config.stylesheet.as_ref()?;

    let relative_href = iter::repeat("../")
        .take(output_file_relative.components().count() - 1)
        .chain(iter::once("style.css"))
        .collect::<String>();

    Some(format!(r#"<link rel="stylesheet" href="{relative_href}">"#))
}

trait StringContents {
    fn get_string_contents(&self) -> String;
}

impl StringContents for mdast::Node {
    fn get_string_contents(&self) -> String {
        match self {
            Self::Text(e) => e.value.clone(),
            Self::InlineCode(e) => e.value.clone(),
            Self::InlineMath(e) => e.value.clone(),
            Self::Link(e) => e.children.get_string_contents(),
            Self::Paragraph(e) => e.children.get_string_contents(),
            Self::Emphasis(e) => e.children.get_string_contents(),
            Self::Strong(e) => e.children.get_string_contents(),
            _ => String::new(),
        }
    }
}

impl StringContents for Vec<mdast::Node> {
    fn get_string_contents(&self) -> String {
        self.iter().map(mdast::Node::get_string_contents).collect()
    }
}

trait FileContents {
    fn get_file_contents(&self) -> Option<String>;
}

impl FileContents for Option<&PathBuf> {
    fn get_file_contents(&self) -> Option<String> {
        let file = *self;
        if let Some(file) = file {
            let result = fs::read_to_string(file);
            return result.is_ok().then(|| result.unwrap());
        }
        None
    }
}

impl FileContents for PathBuf {
    fn get_file_contents(&self) -> Option<String> {
        let result = fs::read_to_string(self);
        result.is_ok().then(|| result.unwrap())
    }
}

impl FileContents for String {
    fn get_file_contents(&self) -> Option<String> {
        let result = fs::read_to_string(self);
        result.is_ok().then(|| result.unwrap())
    }
}
