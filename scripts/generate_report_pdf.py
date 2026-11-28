import base64
import json
import re
import subprocess
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
REPORT_PATH = ROOT_DIR / "REPORT.md"
OUTPUT_HTML = ROOT_DIR / "docs" / "report_rendered.html"
OUTPUT_PDF = ROOT_DIR / "REPORT.pdf"
DOCS_PDF = ROOT_DIR / "docs" / "REPORT.pdf"

HTML_TEMPLATE = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Tsuki // Engineering & Architecture Monograph</title>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/5.5.1/github-markdown-light.min.css">
<script src="https://cdn.jsdelivr.net/npm/marked@12.0.0/marked.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/mermaid@10.9.0/dist/mermaid.min.js"></script>
<style>
    @import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,600;1,6..72,400&family=Space+Grotesk:wght@500;600;700&display=swap');

    @page {
        size: A4;
        margin: 9mm 12mm 9mm 12mm;
    }
    body {
        background-color: #ffffff;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
        color: #141413;
        margin: 0;
        padding: 0;
    }
    .markdown-body {
        box-sizing: border-box;
        max-width: 100%;
        margin: 0 auto;
        padding: 2px;
        font-size: 12.5px;
        line-height: 1.50;
    }
    .page-break {
        page-break-before: always;
        break-before: page;
        height: 0;
        margin: 0;
        padding: 0;
    }
    .markdown-body h1 {
        border-bottom: 2px solid #006B44;
        padding-bottom: 5px;
        color: #141413;
        font-family: 'Space Grotesk', sans-serif;
        font-size: 22px;
        font-weight: 700;
        margin-top: 4px;
        margin-bottom: 10px;
    }
    .markdown-body h2 {
        border-bottom: 1px solid #DEDCD4;
        padding-bottom: 4px;
        margin-top: 14px;
        margin-bottom: 6px;
        color: #141413;
        font-family: 'Space Grotesk', sans-serif;
        font-size: 17px;
        font-weight: 600;
        page-break-after: avoid;
    }
    .markdown-body h3 {
        color: #006B44;
        font-family: 'Space Grotesk', sans-serif;
        font-size: 14.5px;
        font-weight: 600;
        margin-top: 14px;
        margin-bottom: 6px;
        page-break-after: avoid;
    }
    .markdown-body h4 {
        color: #2A2A28;
        font-family: 'Space Grotesk', sans-serif;
        font-size: 12.5px;
        font-weight: 600;
        margin-top: 10px;
        margin-bottom: 3px;
        page-break-after: avoid;
    }
    .markdown-body table {
        width: 100%;
        border-collapse: collapse;
        margin: 10px 0;
        font-size: 10px;
        line-height: 1.35;
        font-family: 'JetBrains Mono', monospace;
        page-break-inside: auto;
    }
    .markdown-body th, .markdown-body td {
        padding: 4px 6px;
        border: 1px solid #DEDCD4;
    }
    .markdown-body th {
        background-color: #F1EFEA;
        font-weight: 600;
        color: #141413;
    }
    .markdown-body tr {
        page-break-inside: avoid;
    }
    .markdown-body pre {
        background-color: #FBFBF9;
        border: 1px solid #DEDCD4;
        border-radius: 4px;
        font-family: 'JetBrains Mono', monospace;
        font-size: 10.5px;
        line-height: 1.42;
        padding: 8px 12px;
        margin: 8px 0;
        page-break-inside: avoid;
    }
    .markdown-body code {
        font-family: 'JetBrains Mono', monospace;
        background-color: #F1EFEA;
        padding: 1px 4px;
        border-radius: 3px;
        font-size: 11px;
    }
    .markdown-body pre code {
        background-color: transparent;
        padding: 0;
    }
    .markdown-body blockquote {
        border-left: 3px solid #006B44;
        color: #2A2A28;
        background-color: #F7F6F2;
        padding: 8px 12px;
        margin: 8px 0 14px 0;
        font-size: 11.5px;
        border-radius: 0 4px 4px 0;
        page-break-inside: avoid;
    }
    .markdown-body img {
        max-width: 86%;
        max-height: 360px;
        width: auto;
        height: auto;
        border: 1px solid #DEDCD4;
        border-radius: 5px;
        box-shadow: 0 3px 8px rgba(0,0,0,0.06);
        margin: 10px auto 4px auto;
        display: block;
        page-break-inside: avoid;
    }
    .mermaid {
        background-color: #FBFBF9;
        border: 1px solid #DEDCD4;
        border-radius: 5px;
        padding: 8px;
        margin: 10px 0;
        text-align: center;
        page-break-inside: avoid;
    }
    .mermaid svg {
        max-width: 88% !important;
        max-height: 340px !important;
        height: auto !important;
    }
    hr {
        height: 1px;
        background-color: #DEDCD4;
        border: none;
        margin: 14px 0;
    }
    ul, ol {
        padding-left: 20px;
        margin: 6px 0;
    }
    li {
        margin-bottom: 2px;
    }
    /* Two-column layout for Table of Contents */
    #content > ul:first-of-type {
        column-count: 2;
        column-gap: 20px;
        font-size: 11px;
        line-height: 1.45;
    }
</style>
</head>
<body class="markdown-body">
<div id="content"></div>

<script>
    mermaid.initialize({
        startOnLoad: false,
        theme: 'neutral',
        flowchart: { useMaxWidth: true, htmlLabels: true, curve: 'basis' }
    });

    const rawMd = `__ESCAPED_MD__`;
    const mermaidDiagrams = __MERMAID_JSON__;

    document.getElementById('content').innerHTML = marked.parse(rawMd);

    // Inject protected mermaid blocks
    mermaidDiagrams.forEach((code, idx) => {
        const placeholder = document.getElementById(`mermaid-placeholder-${idx}`);
        if (placeholder) {
            const div = document.createElement('div');
            div.className = 'mermaid';
            div.textContent = code;
            placeholder.replaceWith(div);
        }
    });

    async function initMermaid() {
        try {
            await mermaid.run({
                nodes: document.querySelectorAll('.mermaid')
            });
        } catch (e) {
            console.error('Mermaid render error:', e);
        }
    }
    initMermaid();
</script>
</body>
</html>
"""

def encode_image_to_base64(img_path: Path) -> str:
    """Read an image and convert it to a data URI string."""
    if not img_path.exists():
        print(f"Warning: Image file not found: {img_path}")
        return ""
    data = img_path.read_bytes()
    b64_str = base64.b64encode(data).decode("ascii")
    suffix = img_path.suffix.lstrip(".").lower()
    mime = "image/png" if suffix == "png" else f"image/{suffix}"
    return f"data:{mime};base64,{b64_str}"

def build_pdf():
    OUTPUT_HTML.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "r", encoding="utf-8") as f:
        md_content = f.read()

    # Encode all screenshots to inline Base64 data URIs
    screenshot_dir = ROOT_DIR / "docs" / "screenshots"
    for img_file in screenshot_dir.glob("*.png"):
        b64_uri = encode_image_to_base64(img_file)
        if b64_uri:
            md_content = md_content.replace(f"docs/screenshots/{img_file.name}", b64_uri)
            md_content = md_content.replace(f"screenshots/{img_file.name}", b64_uri)
            md_content = md_content.replace(f"./docs/screenshots/{img_file.name}", b64_uri)

    # Extract mermaid diagrams to protect them from marked parser
    mermaid_blocks = []
    def save_mermaid(match):
        idx = len(mermaid_blocks)
        mermaid_blocks.append(match.group(1).strip())
        return f'<div id="mermaid-placeholder-{idx}"></div>'

    md_content = re.sub(r'```mermaid\s*\n(.*?)\n```', save_mermaid, md_content, flags=re.DOTALL)

    # Clean LaTeX math notation to crisp Unicode for print rendering
    md_content = md_content.replace(r"\longrightarrow", "⟶")
    md_content = md_content.replace(r"\Delta t_{\text{stalled}}", "Δt(stalled)")
    md_content = md_content.replace(r"\Delta t_{\text{some\_stalled}}", "Δt(some_stalled)")
    md_content = md_content.replace(r"\Delta t_{\text{all\_stalled}}", "Δt(all_stalled)")
    md_content = md_content.replace(r"\Delta t_{\text{wall}}", "Δt(wall)")
    md_content = md_content.replace(r"\text{PSI}_{\text{some}}", "PSI(some)")
    md_content = md_content.replace(r"\text{PSI}_{\text{full}}", "PSI(full)")
    md_content = md_content.replace(r"\text{Beta}(\alpha_k, \beta_k)", "Beta(α_k, β_k)")
    md_content = md_content.replace(r"\text{Beta}(\alpha=124, \beta=8)", "Beta(α=124, β=8)")
    md_content = md_content.replace(r"\alpha=124, \beta=8", "α=124, β=8")
    md_content = md_content.replace(r"\theta_k \sim", "θ_k ~")
    md_content = md_content.replace(r"\le 0.5\%", "≤ 0.5%")
    md_content = md_content.replace(r"\le 1.0\%", "≤ 1.0%")
    md_content = md_content.replace(r"\mathcal{O}(1)", "O(1)")
    md_content = md_content.replace(r"y_t = (x *_d f)(t) = \sum_{i=0}^{k-1} f(i) \cdot x_{t - d \cdot i}", "y_t = (x *_d f)(t) = Σ f(i) · x_{t - d·i}")
    md_content = md_content.replace(r"R = 1 + \sum_{l=0}^{L-1} (k_l - 1) \cdot d_l", "R = 1 + Σ (k_l - 1) · d_l")
    md_content = md_content.replace(r"\Delta \mathcal{R}(c) = \text{Risk}(\text{System}) - \mathbb{E}[\text{Risk}(\text{System} \setminus c)]", "ΔR(c) = Risk(System) - E[Risk(System \\ c)]")
    md_content = md_content.replace(r"\Delta \mathcal{R}(c)", "ΔR(c)")
    md_content = md_content.replace(r"\tau_{\text{isolate}}", "τ(isolate)")
    md_content = md_content.replace(r"\text{watermark}[\text{WMARK\_MIN}]", "watermark[WMARK_MIN]")
    md_content = md_content.replace(r"\text{watermark}[\text{WMARK\_LOW}]", "watermark[WMARK_LOW]")
    md_content = md_content.replace(r"\text{watermark}[\text{WMARK\_HIGH}]", "watermark[WMARK_HIGH]")
    md_content = md_content.replace(r"\frac{\text{min\_free\_kbytes}}{4}", "min_free_kbytes / 4")
    md_content = md_content.replace(r"\times \frac{5}{4}", "× 5/4")
    md_content = md_content.replace(r"\times \frac{3}{2}", "× 3/2")
    md_content = md_content.replace(r"v_{\text{pressure}}(t)", "v_pressure(t)")
    md_content = md_content.replace(r"\text{VmLck} = \text{VmRSS}", "VmLck = VmRSS")

    # Escape backticks and backslashes for JS string template
    escaped_md = md_content.replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${")

    html_content = HTML_TEMPLATE.replace("__ESCAPED_MD__", escaped_md).replace("__MERMAID_JSON__", json.dumps(mermaid_blocks))

    with open(OUTPUT_HTML, "w", encoding="utf-8") as f:
        f.write(html_content)

    print(f"Generated HTML intermediate at: {OUTPUT_HTML} (size: {OUTPUT_HTML.stat().st_size / 1024:.1f} KB)")

    chrome_cmd = [
        "google-chrome",
        "--headless",
        "--no-sandbox",
        "--disable-gpu",
        "--allow-file-access-from-files",
        "--run-all-compositor-stages-before-draw",
        "--virtual-time-budget=9000",
        f"--print-to-pdf={OUTPUT_PDF}",
        "--no-pdf-header-footer",
        str(OUTPUT_HTML.resolve()),
    ]

    print("Running headless Chrome to render PDF...")
    result = subprocess.run(chrome_cmd, capture_output=True, text=True)
    if result.returncode == 0 and OUTPUT_PDF.exists():
        import shutil
        shutil.copy(OUTPUT_PDF, DOCS_PDF)
        size_mb = OUTPUT_PDF.stat().st_size / (1024 * 1024)
        print(f"Successfully generated PDF: {OUTPUT_PDF} ({size_mb:.2f} MB)")
        print(f"Copied to: {DOCS_PDF}")
    else:
        print(f"Error generating PDF. Return code: {result.returncode}, stderr: {result.stderr}, stdout: {result.stdout}")

if __name__ == "__main__":
    build_pdf()
