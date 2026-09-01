// Task Discovery: maps task descriptions to recommended MCP servers, skills, and tech stacks.
// Used by POST /agent/recommend-tools and the recommend_tools MCP tool.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRec {
    pub name: String,
    pub install: String,
    pub use_for: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    pub task_category: String,
    pub mcp_servers: Vec<McpServerRec>,
    pub skills: Vec<String>,
    pub tech_stack: Vec<String>,
    pub setup_commands: Vec<String>,
    pub rationale: String,
    pub react_goal_template: String,
}

fn mcp(name: &str, install: &str, use_for: &str) -> McpServerRec {
    McpServerRec {
        name: name.into(),
        install: install.into(),
        use_for: use_for.into(),
    }
}

pub fn recommend(task: &str) -> ToolRecommendation {
    let t = task.to_lowercase();

    let category = detect_category(&t);
    build_recommendation(category)
}

fn detect_category(t: &str) -> &'static str {
    let web_terms = ["scrape", "playwright", "hotel", "itiner", "flight",
                     "booking.com", "trip.com", "hotel.com", "airbnb", "research price",
                     "price compar", "travel cost", "kyoto", "browse web",
                     "crawl", "fetch url", "web search"];
    let dev_terms = ["full stack", "full-stack", "facebook", "replica", "clone",
                     "build app", "build web", "build api", "backend", "frontend",
                     "react app", "next.js", "node app", "django", "fastapi", "rest api",
                     "graphql", "authentication", "auth system", "user management"];
    let devops_terms = ["deploy", "docker", "kubernetes", "k8s", "terraform", "ci/cd",
                        "github actions", "helm", "infra", "cloud infra", "aws", "azure", "gcp"];
    let data_terms = ["data analys", "sql query", "dashboard", "data chart", "visuali",
                      "etl pipeline", "dataset", "pandas", "jupyter", "notebook", "csv analys", "parquet"];
    let review_terms = ["code review", "audit", "security scan", "vuln", "pentest", "sast",
                        "lint", "tech debt", "dependency check"];
    let content_terms = ["write blog", "blog post", "article", "content creat", "markdown doc",
                         "presentation", "slide deck", "report writ", "summary doc"];
    let agent_terms = ["multi-agent", "multi agent", "orchestrat", "crewai", "autogen", "langchain",
                       "agent loop", "agent harness", "agent system"];
    // trip/travel is web_research but requires broader pattern to avoid matching "trip" in non-travel
    let travel_terms = ["trip plan", "trip from", "trip to", "travel plan", "travel budget",
                        "budget trip", "vacation plan", "holiday plan"];

    for t2 in web_terms    { if t.contains(t2) { return "web_research"; } }
    for t2 in travel_terms { if t.contains(t2) { return "web_research"; } }
    for t2 in dev_terms    { if t.contains(t2) { return "full_stack_dev"; } }
    for t2 in devops_terms { if t.contains(t2) { return "devops"; } }
    for t2 in data_terms   { if t.contains(t2) { return "data_analysis"; } }
    for t2 in review_terms { if t.contains(t2) { return "code_review"; } }
    for t2 in agent_terms  { if t.contains(t2) { return "agent_orchestration"; } }
    for t2 in content_terms{ if t.contains(t2) { return "content_creation"; } }
    "general"
}

fn build_recommendation(category: &str) -> ToolRecommendation {
    match category {
        "web_research" => ToolRecommendation {
            task_category: "web_research".into(),
            mcp_servers: vec![
                mcp("playwright", "npx @playwright/mcp", "Browser automation, web scraping, dynamic pages"),
                mcp("fetch", "npx @modelcontextprotocol/server-fetch", "Fetch HTML/JSON from URLs"),
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Store/recall scraped data across iterations"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["Playwright".into(), "HipCortex".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
                "npx @playwright/mcp install".into(),
            ],
            rationale: "Web research needs browser automation for dynamic sites. HipCortex stores scraped facts across ReAct iterations so the agent doesn't re-fetch the same pages.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["data_collected","sources_verified","cost_computed"],"max_react_iterations":30}"#.into(),
        },

        "full_stack_dev" => ToolRecommendation {
            task_category: "full_stack_dev".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Goal tracking, belief revision, decision provenance"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "Read/write project files"),
                mcp("github", "npx @modelcontextprotocol/server-github", "Create repos, branches, PRs"),
                mcp("postgres", "npx @modelcontextprotocol/server-postgres", "Database schema management"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["React/Next.js or Vue".into(), "Node/FastAPI/Django".into(), "PostgreSQL".into(), "Docker".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
            ],
            rationale: "Full-stack builds need HipCortex for goal tracking across long sessions. FileSystem MCP for code writes; GitHub MCP for version control; Postgres MCP if DB work is in scope.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["auth_system_tested","core_features_working","deployment_ready"],"max_react_iterations":50}"#.into(),
        },

        "data_analysis" => ToolRecommendation {
            task_category: "data_analysis".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Store analysis findings, track hypotheses"),
                mcp("sqlite", "npx @modelcontextprotocol/server-sqlite", "Query local SQLite databases"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "Read CSV/Parquet files"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["Python".into(), "Pandas".into(), "DuckDB or SQLite".into(), "Matplotlib/Plotly".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
                "pip install pandas duckdb matplotlib".into(),
            ],
            rationale: "Data analysis benefits from HipCortex tracking findings across exploration steps. SQLite MCP for SQL queries; FileSystem MCP for reading data files.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["data_loaded","analysis_complete","insights_stored"],"max_react_iterations":20}"#.into(),
        },

        "devops" => ToolRecommendation {
            task_category: "devops".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Track infrastructure decisions and rollback states"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "Edit Terraform/Helm/Dockerfile configs"),
                mcp("github", "npx @modelcontextprotocol/server-github", "Trigger CI/CD workflows, create PRs"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["Docker".into(), "Kubernetes/Helm".into(), "Terraform".into(), "GitHub Actions".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
            ],
            rationale: "DevOps tasks span many files and services. HipCortex tracks infrastructure state and decisions across sessions, enabling safe rollback reasoning.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["infra_defined","tests_passing","deployment_verified"],"max_react_iterations":30}"#.into(),
        },

        "code_review" => ToolRecommendation {
            task_category: "code_review".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Track findings, patterns, and risk assessments"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "Read source files"),
                mcp("github", "npx @modelcontextprotocol/server-github", "Comment on PRs, query issue history"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["Static analysis tools (clippy/eslint/ruff)".into(), "SAST scanners".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
            ],
            rationale: "Code review benefits from HipCortex storing findings so the agent can cross-reference patterns, track risk, and avoid re-analyzing the same files.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["all_files_reviewed","findings_stored","risk_assessed"],"max_react_iterations":20}"#.into(),
        },

        "agent_orchestration" => ToolRecommendation {
            task_category: "agent_orchestration".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive --actor orchestrator", "Shared substrate for multi-agent memory and goal state"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "Shared workspace for agent outputs"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["LangGraph or CrewAI".into(), "HipCortex MCP".into(), "Python 3.11+".into()],
            setup_commands: vec![
                "pip install hipcortex crewai".into(),
                "hipcortex install --mode proactive --actor orchestrator".into(),
                "hipcortex install --mode proactive --actor worker".into(),
                "hipcortex start".into(),
            ],
            rationale: "Multi-agent systems need a shared substrate. HipCortex provides actor-scoped memory with shared symbolic graph — each agent gets its own namespace but shares world model and beliefs.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["agents_coordinated","tasks_distributed","results_merged"],"max_react_iterations":40}"#.into(),
        },

        "content_creation" => ToolRecommendation {
            task_category: "content_creation".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Store outlines, drafts, and revision history"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "Read/write content files"),
                mcp("fetch", "npx @modelcontextprotocol/server-fetch", "Fetch reference sources"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["Markdown".into(), "Pandoc (PDF/DOCX export)".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
            ],
            rationale: "Long-form content benefits from HipCortex storing the outline, draft sections, and revision decisions — so the agent can resume without re-reading everything.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["outline_complete","draft_written","reviewed"],"max_react_iterations":15}"#.into(),
        },

        _ => ToolRecommendation {
            task_category: "general".into(),
            mcp_servers: vec![
                mcp("hipcortex", "pip install hipcortex && hipcortex install --mode proactive", "Persistent memory, goal tracking, belief revision"),
                mcp("filesystem", "npx @modelcontextprotocol/server-filesystem", "File read/write"),
                mcp("fetch", "npx @modelcontextprotocol/server-fetch", "HTTP resource fetch"),
            ],
            skills: vec!["hipcortex".into()],
            tech_stack: vec!["HipCortex".into()],
            setup_commands: vec![
                "pip install hipcortex".into(),
                "hipcortex install --mode proactive".into(),
                "hipcortex start".into(),
            ],
            rationale: "Start with HipCortex for memory + goal tracking. Add task-specific MCP servers (filesystem, fetch, playwright, github) as the task requirements become clearer.".into(),
            react_goal_template: r#"{"description":"<task>","success_factors":["task_complete"],"max_react_iterations":20}"#.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_research_kyoto() {
        let rec = recommend("Plan a 7-day Kyoto sakura trip with hotel costs and flights from Singapore");
        assert_eq!(rec.task_category, "web_research");
        assert!(rec.mcp_servers.iter().any(|s| s.name == "playwright"));
        assert!(!rec.setup_commands.is_empty());
        assert!(rec.react_goal_template.contains("success_factors"));
    }

    #[test]
    fn test_full_stack_dev_facebook() {
        let rec = recommend("Build a full-stack Facebook replica with auth, news feed, and profiles");
        assert_eq!(rec.task_category, "full_stack_dev");
        assert!(rec.mcp_servers.iter().any(|s| s.name == "hipcortex"));
        assert!(rec.mcp_servers.iter().any(|s| s.name == "github"));
        assert!(rec.react_goal_template.contains("auth_system_tested"));
    }

    #[test]
    fn test_data_analysis() {
        let rec = recommend("Analyse our sales CSV data and produce a dashboard");
        assert_eq!(rec.task_category, "data_analysis");
        assert!(rec.mcp_servers.iter().any(|s| s.name.contains("sqlite") || s.name.contains("filesystem")));
    }

    #[test]
    fn test_devops() {
        let rec = recommend("Deploy our app to Kubernetes with Helm charts");
        assert_eq!(rec.task_category, "devops");
        assert!(rec.tech_stack.iter().any(|s| s.contains("Kubernetes")));
    }

    #[test]
    fn test_code_review() {
        let rec = recommend("Do a security audit of the auth module");
        assert_eq!(rec.task_category, "code_review");
        assert!(rec.mcp_servers.iter().any(|s| s.name == "github"));
    }

    #[test]
    fn test_general_fallback() {
        let rec = recommend("Help me think through my startup idea");
        assert_eq!(rec.task_category, "general");
        assert!(rec.mcp_servers.iter().any(|s| s.name == "hipcortex"));
    }
}
