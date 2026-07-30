let meta = #{
    name: "gsa-retrieval",
    description: "GSA parallel retrieval: project docs + external search, each with neutral completion check",
    phases: [
        #{ title: "Retrieve", detail: "project-doc and external retrieval agents in parallel" },
    ],
};

// -- schemas -----------------------------------------------------------
let retrieval_output = #{
    "type": "object",
    "required": ["retrieval_profile", "sources_checked", "relevant_findings",
                 "missing_or_uncertain", "completion_check"],
    "properties": #{
        "retrieval_profile": #{ "type": "string" },
        "sources_checked": #{ "type": "array", "items": #{ "type": "string" } },
        "relevant_findings": #{ "type": "string" },
        "missing_or_uncertain": #{ "type": "string" },
        "completion_check": #{ "type": "string" },
    },
};

// -- args guard ---------------------------------------------------------
let query = if args == () { () } else { args.query };
if query == () { pause("verification", "Pass args.query with the retrieval question."); }

// -- parallel retrieval -------------------------------------------------
phase("Retrieve");
let jobs = [
    #{
        prompt: "Retrieve project documentation for: " + query + ". "
            + "Use Read, Grep, and Glob tools to search the project workspace. "
            + "Start from CLI_PROJECT_INDEX.md for routing, then follow references. "
            + "Return concrete findings with exact file paths and the reason each source is relevant. "
            + "Report any gaps or uncertain areas. "
            + "Answer the neutral completion check: has the material needed for this task been obtained?",
        label: "project-docs:" + query,
        agent_type: "gsa-project-doc-retrieval",
        capability_mode: "read-only",
        output_schema: retrieval_output,
    },
    #{
        prompt: "Retrieve external information for: " + query + ". "
            + "Use web_search and web_fetch tools to find relevant external sources. "
            + "Prefer source URLs, page metadata, stable identifiers, and downloadable PDFs. "
            + "Return evidence candidates with URLs. "
            + "Report access limits, paywalls, blocked pages, or unresolved provenance. "
            + "Answer the neutral completion check: has the material needed for this task been obtained?",
        label: "external:" + query,
        agent_type: "gsa-external-retrieval",
        capability_mode: "read-only",
        output_schema: retrieval_output,
    },
];

let results = parallel(jobs);

// -- collect findings ---------------------------------------------------
let project_findings = ();
let external_findings = ();
let project_complete = false;
let external_complete = false;

if results[0] != () && results[0].success && results[0].output != () {
    project_findings = results[0].output;
    let cc = project_findings.completion_check;
    project_complete = cc != () && cc != "";
}
if results[1] != () && results[1].success && results[1].output != () {
    external_findings = results[1].output;
    let cc = external_findings.completion_check;
    external_complete = cc != () && cc != "";
}

// -- merge --------------------------------------------------------------
let project_sources = if project_findings != () && project_findings.sources_checked != () {
    project_findings.sources_checked.len()
} else { 0 };
let external_sources = if external_findings != () && external_findings.sources_checked != () {
    external_findings.sources_checked.len()
} else { 0 };

let summary = "Project sources: " + project_sources.to_string()
    + ", external sources: " + external_sources.to_string()
    + ". Project complete: " + project_complete.to_string()
    + ", external complete: " + external_complete.to_string();

log(summary);

complete(#{
    summary: summary,
    query: query,
    project_retrieval: project_findings,
    external_retrieval: external_findings,
    neutral_completion: #{
        project_material_obtained: project_complete,
        external_material_obtained: external_complete,
    },
});
