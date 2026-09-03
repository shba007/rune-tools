### `mhb-mconnect`

* **Description:** A Modest Human Brands (MHB) external interaction API connector MCP server that manages email templates over a REST backend. Lists all available email templates, fetches a specific template definition along with its required variable schema by ID, and renders a fully styled HTML email preview from supplied variables. All requests are proxied through a native sidecar (`mhb-mconnect-native`) targeting the configured MHB base URL.

* **Tool Definitions:** `mhb_list_templates`, `mhb_get_template`, `mhb_render_template_preview`

* **MCP Configuration:**

```json
{
  "mcpServers": {
    "mhb-mconnect": {
      "command": "rune",
      "args": [
        "run",
        "mhb-mconnect"
      ],
      "env": {
        "MHB_BASE_URL": "https://example.com"
      }
    }
  }
}
```

**Environment Variables:**

* `MHB_BASE_URL`: Base URL of the MHB interaction API that all template endpoints are prefixed with (default: `https://api.modesthumanbrands.com`). May also be supplied per-call via a `baseUrl` argument.

#### Use Case MHB-01: List All Available Templates

* **Category:** Happy Path / Query
* **Prompt:** "What email templates do I have available in the MHB service?"
* **Expected Tool(s):** `mhb_list_templates`

#### Use Case MHB-02: Inspect a Template's Variable Schema

* **Category:** Happy Path / Detail
* **Prompt:** "Show me the definition and required variables for the 'internship-completion-certificate' template."
* **Expected Tool(s):** `mhb_get_template`

#### Use Case MHB-03: Render an HTML Preview

* **Category:** Happy Path / Rendering
* **Prompt:** "Render a preview of the 'welcome-email' template with recipient name 'Aarav' and company 'Acme Corp'."
* **Expected Tool(s):** `mhb_render_template_preview`

#### Use Case MHB-04: Custom Base URL Override

* **Category:** Granular Options / Configuration
* **Prompt:** "List the templates, but hit my staging instance at https://staging.modesthumanbrands.com instead of production."
* **Expected Tool(s):** `mhb_list_templates`

#### Use Case MHB-05: Missing Template ID Validation

* **Category:** Edge Case / Validation
* **Prompt:** "Fetch the template with no ID given." (missing required `templateId`)
* **Expected Tool(s):** `mhb_get_template`
