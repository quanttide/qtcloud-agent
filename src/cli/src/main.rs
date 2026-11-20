mod audit;
mod registry;
mod session;
mod util;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// 量潮智能体云 CLI：会话取用与注册中心对照
#[derive(Parser)]
#[command(name = "qtcloud-agent", version, about = "会话取用与注册中心对照")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 注册中心：与本地 agent 配置对照
    #[command(subcommand)]
    Registry(RegistryCommand),
    /// 会话：找出来、看清楚、按归属筛、交出去前先审计
    #[command(subcommand)]
    Session(SessionCommand),
}

#[derive(Subcommand)]
enum RegistryCommand {
    /// 对照注册中心与本地 agent 配置
    Status {
        /// 注册中心路径（默认取 QTCLOUD_AGENT_REGISTRY，回退 data/profile/default/agents）
        #[arg(long)]
        path: Option<PathBuf>,
        /// 逐个列出本地自定义文件
        #[arg(long)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// 列出会话（按记录时的 cwd 判定归属）
    List {
        /// 只列某个 harness 的会话（默认全部）
        #[arg(long, value_enum)]
        agent: Option<Harness>,
        /// 按目录归属筛选（默认当前目录）
        #[arg(long, conflicts_with = "all")]
        cwd: Option<PathBuf>,
        /// 不按目录筛选
        #[arg(long)]
        all: bool,
        /// 输出 JSON
        #[arg(long)]
        json: bool,
    },
    /// 看一个会话的详情
    Show {
        /// 会话 id
        id: String,
        /// 只在某个 harness 里找
        #[arg(long, value_enum)]
        agent: Option<Harness>,
        /// 输出 JSON
        #[arg(long)]
        json: bool,
    },
    /// 审计目录里的敏感串（只读，命中即非零退出）
    Audit {
        /// 目录或文件
        path: PathBuf,
        /// 输出 JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Harness {
    /// DeepSeek Harness（$DSH_HOME，默认 ~/.dsh）
    Dsh,
    /// pi（~/.pi/agent/sessions）
    Pi,
    /// OpenCode（~/.local/share/opencode/opencode.db）
    Opencode,
    /// Hermes（~/.hermes/state.db）
    Hermes,
}

impl Harness {
    fn name(self) -> &'static str {
        match self {
            Harness::Dsh => "dsh",
            Harness::Pi => "pi",
            Harness::Opencode => "opencode",
            Harness::Hermes => "hermes",
        }
    }
}

const ALL: [&str; 4] = ["dsh", "pi", "opencode", "hermes"];

fn main() {
    let code = match Cli::parse().command {
        Command::Registry(cmd) => match cmd {
            RegistryCommand::Status { path, verbose } => {
                registry::status(&registry_path(path), verbose)
            }
        },
        Command::Session(cmd) => match cmd {
            SessionCommand::List {
                agent,
                cwd,
                all,
                json,
            } => {
                let agents: Vec<&str> = match agent {
                    Some(h) => vec![h.name()],
                    None => ALL.to_vec(),
                };
                let target = if all {
                    None
                } else {
                    Some(cwd.unwrap_or_else(current_dir))
                };
                session::list(&agents, target.as_deref(), all, json)
            }
            SessionCommand::Show { id, agent, json } => {
                let agents: Vec<&str> = match agent {
                    Some(h) => vec![h.name()],
                    None => ALL.to_vec(),
                };
                session::show(&agents, &id, json)
            }
            SessionCommand::Audit { path, json } => audit::run(&path, json),
        },
    };
    std::process::exit(code);
}

fn current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn registry_path(cli: Option<PathBuf>) -> PathBuf {
    cli.or_else(|| std::env::var_os("QTCLOUD_AGENT_REGISTRY").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("data/profile/default/agents"))
}
