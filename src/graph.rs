use std::fmt::Write as _;
use std::path::PathBuf;

use clap::Parser;

use crate::config;
use crate::config::Config;

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case")]
pub struct Opts {
    /// Read configuration from one or more files. Wildcard paths are supported.
    /// File format is detected from the file name.
    /// If zero files are specified the default config path
    /// `/etc/vector/vector.toml` will be targeted.
    #[arg(
        id = "config",
        short,
        long,
        env = "VECTOR_CONFIG",
        value_delimiter(',')
    )]
    paths: Vec<PathBuf>,

    /// Vector config files in TOML format.
    #[arg(id = "config-toml", long, value_delimiter(','))]
    paths_toml: Vec<PathBuf>,

    /// Vector config files in JSON format.
    #[arg(id = "config-json", long, value_delimiter(','))]
    paths_json: Vec<PathBuf>,

    /// Vector config files in YAML format.
    #[arg(id = "config-yaml", long, value_delimiter(','))]
    paths_yaml: Vec<PathBuf>,

    /// Read configuration from files in one or more directories.
    /// File format is detected from the file name.
    ///
    /// Files not ending in .toml, .json, .yaml, or .yml will be ignored.
    #[arg(
        id = "config-dir",
        short = 'C',
        long,
        env = "VECTOR_CONFIG_DIR",
        value_delimiter(',')
    )]
    pub config_dirs: Vec<PathBuf>,

    /// Select output format of a graph.
    /// By default DOT (Graphviz).
    /// You can choose: dot, mermaid
    /// Generated contents printed to stdout.
    #[arg(
        id = "output-format",
        short = 'f',
        long
    )]
    pub output_format: String,

}

impl Opts {
    fn paths_with_formats(&self) -> Vec<config::ConfigPath> {
        config::merge_path_lists(vec![
            (&self.paths, None),
            (&self.paths_toml, Some(config::Format::Toml)),
            (&self.paths_json, Some(config::Format::Json)),
            (&self.paths_yaml, Some(config::Format::Yaml)),
        ])
        .map(|(path, hint)| config::ConfigPath::File(path, hint))
        .chain(
            self.config_dirs
                .iter()
                .map(|dir| config::ConfigPath::Dir(dir.to_path_buf())),
        )
        .collect()
    }
    fn output_format(self) -> String {
        self.output_format
    }
}

pub(crate) fn cmd(opts: &Opts) -> exitcode::ExitCode {
    let paths = opts.paths_with_formats();
    let paths = match config::process_paths(&paths) {
        Some(paths) => paths,
        None => return exitcode::CONFIG,
    };

    let config = match config::load_from_paths(&paths) {
        Ok(config) => config,
        Err(errs) => {
            #[allow(clippy::print_stderr)]
            for err in errs {
                eprintln!("{}", err);
            }
            return exitcode::CONFIG;
        }
    };

    let mut graph = String::from("");
    let output_format = opts.output_format();
    if output_format == "dot" {
        graph = graphviz_graph(config)
    } else {
        if output_format == "mermaid" {
            graph = mermaid_graph(config)
        }
    }



    #[allow(clippy::print_stdout)]
    {
        println!("{}", graph);
    }

    exitcode::OK
}


fn graphviz_graph(config: Config) -> String {
    let mut dot = String::from("digraph {\n");

    for (id, _source) in config.sources() {
        writeln!(dot, "  \"{}\" [shape=trapezium]", id).expect("write to String never fails");
    }

    for (id, transform) in config.transforms() {
        writeln!(dot, "  \"{}\" [shape=diamond]", id).expect("write to String never fails");

        for input in transform.inputs.iter() {
            if let Some(port) = &input.port {
                writeln!(
                    dot,
                    "  \"{}\" -> \"{}\" [label=\"{}\"]",
                    input.component, id, port
                )
                .expect("write to String never fails");
            } else {
                writeln!(dot, "  \"{}\" -> \"{}\"", input, id)
                    .expect("write to String never fails");
            }
        }
    }

    for (id, sink) in config.sinks() {
        writeln!(dot, "  \"{}\" [shape=invtrapezium]", id).expect("write to String never fails");

        for input in &sink.inputs {
            if let Some(port) = &input.port {
                writeln!(
                    dot,
                    "  \"{}\" -> \"{}\" [label=\"{}\"]",
                    input.component, id, port
                )
                .expect("write to String never fails");
            } else {
                writeln!(dot, "  \"{}\" -> \"{}\"", input, id)
                    .expect("write to String never fails");
            }
        }
    }

    dot += "}";
    dot
}

fn mermaid_graph(config: Config) -> String {
    let mut mm = String::from("flowchart TD\n");

    for (id, _source) in config.sources() {
        writeln!(mm, "  {}[/{}\\]", id, id).expect("write to String never fails");
    }

    for (id, transform) in config.transforms() {
        writeln!(mm, "  {}[{{ {} }}]", id, id).expect("write to String never fails");

        for input in transform.inputs.iter() {
            if let Some(port) = &input.port {
                writeln!(
                    mm,
                    "  {}--{}-->{}", // label in the middle
                    input.component, port, id
                )
                .expect("write to String never fails");
            } else {
                writeln!(mm, "  {}-->{}", input, id)
                    .expect("write to String never fails");
            }
        }
    }

    for (id, sink) in config.sinks() {
        writeln!(mm, "  {}[\\ {} /]", id, id).expect("write to String never fails");

        for input in &sink.inputs {
            if let Some(port) = &input.port {
                writeln!(
                    mm,
                    "  {}--{}-->{}",
                    input.component, port, id
                )
                .expect("write to String never fails");
            } else {
                writeln!(mm, "  {}-->{}", input, id)
                    .expect("write to String never fails");
            }
        }
    }
    mm
}
