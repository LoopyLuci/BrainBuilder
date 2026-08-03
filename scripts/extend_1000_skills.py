#!/usr/bin/env python3
"""Generate a comprehensive set of Hermes skills for ALL categories."""
from pathlib import Path
import re

SKILLS_DIR = Path("C:/Users/limpi/AppData/Local/hermes/skills")
TARGET = 50

EXTENSIONS = {
    "apple": ["apple-device-management","apple-enterprise","apple-education","apple-accessibility","apple-healthkit","apple-ml","apple-silicon","apple-watch","apple-vision","apple-business-chat","apple-arcade","apple-music-api","apple-news","apple-pay","apple-developer","apple-testflight","apple-xcode","apple-swiftui","apple-catalyst","apple-coreml","apple-foundation","apple-health-records","apple-homekit","apple-icloud","apple-identity","apple-in-store","apple-mapkit","apple-network-extension","apple-pencil","apple-safari","apple-search-ads","apple-tvos","apple-watchos","apple-widgets","apple-wwdc","apple-xr","apple-zip","app-store","app-store-connect","app-store-reviews","app-store-subscriptions","app-store-server","apple-configurator","apple-education-api","apple-enterprise-connect","apple-fitness-plus","apple-media","apple-passwords","apple-reminders","apple-shortcuts","apple-tips","apple-tv-plus","apple-wallet","apple-workspace"],
    "autonomous-ai-agents": ["agent-communication","agent-delegation","agent-discovery","agent-health","agent-identity","agent-observability","agent-orchestration","agent-policy","agent-recovery","agent-replay","agent-routing","agent-sandbox","agent-security","agent-telemetry","agent-versioning","autonomous-agents","crewai-orchestration","langgraph-agents","multi-agent","openai-agents","semantic-kernel","tool-using-agents","agent-memory","agent-runtime","agent-debugger","agent-marketplace","agent-scheduler","agent-billing","agent-compliance","agent-metrics","agent-logs","agent-registry","agent-ssl","agent-auth","agent-cron","agent-backup","agent-restore","agent-cluster","agent-shutdown","agent-startup","agent-restart","agent-upgrade","agent-downgrade","agent-rollback","agent-migration","agent-test","agent-staging","agent-prod"],
    "creative": ["aesthetic-evaluation","art-history-critique","art-market","art-trends","artist-collab","brand-storytelling","canvas-composition","ceramics-design","choreography","cinematography","color-harmony","comic-creation","creative-process","critique-guild","cultural-critique","design-thinking","digital-collage","drawing-tutor","exhibition-design","film-scoring","gallery-ops","graphic-poetry","immersive-story","installation-art","jazz-ai","landscape-painting","light-art","manga-ai","mixed-media","music-critique","narrative-ai","nft-art","painting-restoration","papercraft","performance-art","photo-journalism","poetry-critique","pop-art","portrait-ai","printmaking","public-art","sculpture-ai","sound-art","stained-glass","street-art","textile-art","typography","video-art","visual-effects"],
    "data-analysis": ["anomaly-detection","bayesian-analysis","causal-impact","cohort-analysis","correlation","cross-validation","data-profiling","distribution-fit","effect-size","factor-analysis","forecasting","geospatial-analysis","hypothesis-test","interquartile","kernel-density","linear-regression","logistic-regression","manova","markov-chains","monte-carlo","moving-average","multivariate","nonparametric","outlier-score","pca","percentile","permutation-test","power-analysis","probability","quantile-regression","random-forest","seasonality","sensitivity-analysis","serial-correlation","signal-processing","spatial-autocorrelation","spectral-analysis","survival-analysis","time-series","trend-analysis","variance","wavelet","weighted-average","wilcoxon","z-score"],
    "devops": ["artifact-cache","artifact-pruning","blue-green","build-pipeline","canary","chaos","ci-cache","config-repo","deploy-approval","drift-detection","env-parity","gitops","hook-validation","incident-runbook","infra-drift","lb-probe","log-rotation","merge-queue","nightly-build","patch-tuesday","queue-depth","release-train","rollback","secret-sync","slack-notify","stale-branch","statuspage","tagging","terraform-modules","vault-integration","workflow-cache","zone-failover"],
    "github": ["actions-cache","actions-matrix","branch-protection","codeowners","dependabot","discussions-ops","issue-triage","labels","milestones","projects-ops","pull-request","readme-generation","release-drafter","repo-archiving","repo-cloning","repo-mirroring","repo-templates","secret-scanning","sponsors","status-checks","vulnerability-alerts","workflow-runs"],
    "research": ["altmetric","bibliography","citation-analysis","data-citation","doi-management","funder-compliance","impact-factor","interdisciplinarity","knowledge-graph","literature-map","metascience","open-data","open-peer-review","preprint-ops","publication-metrics","replication-crisis","research-software","reviewer-finder","scholarly-communication","science-of-science","scientometrics","text-mining-research","viz-research"],
    "frontend": ["a11y-testing","css-architecture","design-systems","dev-ui","e2e-testing","form-validation","hydration","icon-system","layout-engine","micro-frontends","perf-monitoring","responsive-images","route-guards","server-components","state-management","storybook","style-guide","testing-library","theming","viewport","web-vitals","widget-architecture"],
    "integration": ["api-gateway-integration","broker-integration","cdc-integration","contract-testing","data-contract","eda-integration","etl-integration","event-bus","graphql-integration","grpc-integration","legacy-integration","message-queue","pubsub","rest-integration","rpc-integration","schema-evolution","service-mesh","sse-integration","streaming-integration","webhook-integration","ws-integration"],
    "rust": ["async-std","axum","clap","crossbeam","dashmap","enum-optimization","error-handling","generic-programming","iterator-optimization","macro-writing","memory-optimization","mitigations","no-std","panic-handling","pin-project","rayon","reference-cycles","serde-optimization","smart-pointer","stack-optimization","std-patterns","tracing","unsafe-code","vec-optimization"],
    "media": ["audio-format","image-compression","media-metadata","media-pipeline","streaming-protocols","thumbnail-generation","video-encoding","video-streaming"],
    "note-taking": ["flashcard-ai","knowledge-graph","mindmap","note-linking","note-search","note-summarization","outline-ai","spaced-repetition","study-assistant","zettelkasten"],
    "productivity": ["calendar-ai","clipboard-manager","email-summarizer","focus-timer","inbox-zero","meeting-notes","note-summarizer","pomodoro","reading-list","reminder-ai","task-batching","time-blocking","weekly-review"],
    "smart-home": ["device-sync","energy-usage","home-automation","motion-sensors","scene-automation","security-cameras","smart-lighting","smart-thermostat","voice-control","weather-integration"],
    "social-media": ["audience-growth","community-ai","content-strategy","engagement-ai","hashtag-ai","influencer-ops","moderation-ai","sentiment-ai","share-automation","trend-detection"],
    "email": ["attachment-handling","email-filter","email-templates","follow-up-ai","inbox-ops","out-of-office","phishing-detection","priority-inbox","send-time-optimization","thread-summarization","unsubscribe-ai"],
    "tauri-2x-configuration": ["deep-linking","file-associations","multi-window","plugin-architecture","sidecar","single-instance","system-tray","updater","window-state"],
    "testing": ["fuzz-testing","load-testing","property-testing","snapshot-testing","stress-testing","test-automation","test-coverage","test-data","test-doubles","test-environment"],
}

TEMPLATE = """---
name: {name}
description: "{desc}"
version: 1.0.0
author: Hermes Agent
license: MIT
metadata:
  hermes:
    tags: [{tags}]
prerequisites:
---

# {title}

Brief 3-6 line capability description.

## Workflow

1. Discovery step.
2. Validation step.
3. Execution step.

## Quality Gates

- Gate 1.
- Gate 2.
- Gate 3.
"""


def ensure_unique(names):
    seen = {}
    out = []
    for n in names:
        if n in seen:
            seen[n] += 1
            n = f"{n}-{seen[n]}"
        else:
            seen[n] = 0
        out.append(n)
    return out


def main():
    # Load all categories from existing generator
    base = Path(__file__).resolve().parent
    text = (base / "generate_1000_skills.py").read_text(encoding="utf-8")
    cats = re.findall(r'"(software-development|data-engineering|mlops|devops-infrastructure|security-compliance|research-science|robotics-hardware|health-biology|climate-sustainability|education-learning|governance-policy|finance-economics|creative-arts|communication-media|spatial-urban|quantum-computing|space-technology|social-humanities|agriculture-food|manufacturing-industry)":\s*\[([\s\S]*?)\]', text)
    
    # Collect existing categories not in the above
    existing_non_new = {}
    for d in sorted(SKILLS_DIR.iterdir()):
        if d.is_dir():
            name = d.name
            skills = [p.parent.name for p in d.rglob("SKILL.md") if p.is_file()]
            if not any(name == c for c, _ in cats):
                existing_non_new[name] = skills
    
    all_names = []
    lines = ['#!/usr/bin/env python3\n"""Generate all Hermes skills across all categories."""\nfrom pathlib import Path\n\nSKILLS_DIR = Path("C:/Users/limpi/AppData/Local/hermes/skills")\nCATEGORIES = {']
    
    # Add the 20 original categories
    for cat, body in cats:
        skills = re.findall(r'"([a-z0-9-]+)"', body)[:TARGET]
        all_names.extend(skills)
        lines.append(f'    "{cat}": [')
        for s in skills:
            lines.append(f'        "{s}",')
        lines.append('    ],')
    
    # Add other existing categories extended to TARGET
    for cat, skills in existing_non_new.items():
        existing_set = set(skills)
        pool = EXTENSIONS.get(cat, [f"{cat}-skill-{i}" for i in range(1, TARGET+10)])
        filler = [s for s in pool if s not in existing_set][:max(0, TARGET - len(skills))]
        all_names.extend(filler)
        if filler:
            lines.append(f'    "{cat}": [')
            for s in filler:
                lines.append(f'        "{s}",')
            lines.append('    ],')
    
    all_names = ensure_unique(all_names)
    lines.append('}\n')
    lines.append(TEMPLATE)
    Path(base / "generate_1000_skills.py").write_text("\n".join(lines), encoding="utf-8")
    print(f"All-categories script ready for {len(all_names)} skills")

if __name__ == "__main__":
    main()
