/**
 * Example flowchart templates that users can load to learn
 * the visual flowchart builder system.
 */

import type { FlowchartDefinitionDto } from "./tauri-commands";

export interface ExampleFlowchart {
  id: string;
  name: string;
  description: string;
  /** Factory that creates a fresh definition with current timestamps. */
  create: () => FlowchartDefinitionDto;
}

/**
 * McDonald's Job Application Flow
 *
 * Demonstrates: web_search, web_scrape, LLM analysis, condition branching,
 * app_interact (Notepad), set_variable, error handling.
 *
 * Flow:
 *   Trigger → Search Web → Check Results → Scrape Page → LLM Summarize
 *   → Launch Notepad → Wait → Find Editor → Type Note → Output
 */
function createMcdonaldsJobFlow(): FlowchartDefinitionDto {
  const now = new Date().toISOString();
  return {
    id: `flow.example.mcdonalds_apply_${Date.now()}`,
    name: "McDonald's Job Application Finder",
    version: "1.0.0",
    author: "Omni Examples",
    description:
      "Searches the web for McDonald's job applications near a given location, scrapes the application page, uses an LLM to summarize it, then opens Notepad and writes a summary note.",
    enabled: true,
    created_at: now,
    updated_at: now,
    tools: [
      {
        name: "find_and_note",
        description:
          "Search for McDonald's job applications, scrape the results, and write a summary to Notepad",
        parameters: {
          type: "object",
          properties: {
            location: {
              type: "string",
              description:
                "City, state, or zip code to search near (e.g. 'Chicago, IL')",
            },
            applicant_name: {
              type: "string",
              description: "Your name (included in the Notepad note)",
            },
          },
          required: ["location"],
        },
        trigger_node_id: "trigger_1",
      },
    ],
    permissions: [
      {
        capability: "search.web",
        reason: "Search for McDonald's job listings",
        required: true,
      },
      {
        capability: "browser.scrape",
        reason: "Scrape the job application page",
        required: true,
      },
      {
        capability: "ai.inference",
        reason: "Analyze search results and summarize the application page",
        required: true,
      },
      {
        capability: "app.automation",
        reason: "Open Notepad and type the summary note",
        required: true,
      },
    ],
    config: {},
    nodes: [
      // ── Entry ─────────────────────────────────────────
      {
        id: "trigger_1",
        node_type: "trigger",
        label: "Start",
        position: { x: 400, y: 0 },
        config: {},
      },
      {
        id: "comment_intro",
        node_type: "comment",
        label:
          "This flow searches the web for McDonald's jobs, scrapes the top result, summarizes it with an LLM, and writes the summary into Notepad.",
        position: { x: 700, y: 0 },
        config: {},
      },

      // ── Step 1: Search the Web ────────────────────────
      {
        id: "search_web",
        node_type: "native_tool",
        label: "Search for McDonald's Jobs",
        position: { x: 400, y: 120 },
        config: {
          tool_name: "web_search",
          params_json: {
            query:
              "McDonald's job application careers apply near {{$.params.location}}",
            count: 5,
          },
        },
      },

      // ── Step 2: Store search results ──────────────────
      {
        id: "set_results",
        node_type: "set_variable",
        label: "Store Search Results",
        position: { x: 400, y: 240 },
        config: {
          variable_name: "search_results",
          value_expression: "$.nodes.search_web",
        },
      },

      // ── Step 3: Check if results found ────────────────
      {
        id: "check_results",
        node_type: "condition",
        label: "Results Found?",
        position: { x: 400, y: 360 },
        config: {
          expression: "$var.search_results exists",
        },
      },

      // ── True branch: Process results ──────────────────

      // Step 4: Ask LLM to pick the best URL
      {
        id: "llm_pick_url",
        node_type: "llm_request",
        label: "LLM: Pick Best URL",
        position: { x: 250, y: 500 },
        config: {
          prompt_template:
            'From these web search results about McDonald\'s job applications near {{$.params.location}}, extract the single best URL to a McDonald\'s careers/apply page. Return ONLY the URL, nothing else.\n\nSearch results:\n{{$var.search_results}}',
          max_tokens: 200,
        },
      },

      // Step 5: Scrape that URL
      {
        id: "scrape_page",
        node_type: "native_tool",
        label: "Scrape Application Page",
        position: { x: 250, y: 640 },
        config: {
          tool_name: "web_scrape",
          params_json: {
            url: "{{$.nodes.llm_pick_url.response}}",
            mode: "extract",
          },
        },
      },

      // Step 6: LLM summarize the scraped page
      {
        id: "llm_summarize",
        node_type: "llm_request",
        label: "LLM: Summarize Application",
        position: { x: 250, y: 780 },
        config: {
          prompt_template:
            "Summarize this McDonald's job application page for someone looking to apply. Include:\n- Available positions\n- Requirements\n- How to apply (steps)\n- Direct application URL\n- Any key details (pay, hours, benefits)\n\nKeep it concise but informative. Format as a clean text note.\n\nApplicant name: {{$.params.applicant_name}}\nLocation: {{$.params.location}}\n\nPage content:\n{{$.nodes.scrape_page}}",
          max_tokens: 1000,
        },
      },

      // Step 7: Store the summary
      {
        id: "set_summary",
        node_type: "set_variable",
        label: "Store Summary",
        position: { x: 250, y: 920 },
        config: {
          variable_name: "summary",
          value_expression: "$.nodes.llm_summarize.response",
        },
      },

      // Step 8: Build the note text
      {
        id: "build_note",
        node_type: "transform",
        label: "Build Note Text",
        position: { x: 250, y: 1040 },
        config: {
          mode: "template",
          template:
            "=== McDonald's Job Application Notes ===\nDate: {{$.nodes.trigger_1}}\nLocation: {{$.params.location}}\nApplicant: {{$.params.applicant_name}}\n\n{{$var.summary}}\n\n--- Generated by Omni Flowchart ---",
        },
      },

      // Step 9: Launch Notepad
      {
        id: "launch_notepad",
        node_type: "native_tool",
        label: "Launch Notepad",
        position: { x: 250, y: 1160 },
        config: {
          tool_name: "app_interact",
          params_json: {
            action: "launch",
            executable: "notepad.exe",
          },
        },
      },

      // Step 10: Wait for Notepad to open
      {
        id: "wait_notepad",
        node_type: "delay",
        label: "Wait for Notepad",
        position: { x: 250, y: 1280 },
        config: {
          milliseconds: 2000,
        },
      },

      // Step 11: Find the text editor element in Notepad
      {
        id: "find_editor",
        node_type: "native_tool",
        label: "Find Text Editor",
        position: { x: 250, y: 1400 },
        config: {
          tool_name: "app_interact",
          params_json: {
            action: "find_element",
            process_name: "notepad.exe",
            control_type: "Edit",
          },
        },
      },

      // Step 12: Type the note into Notepad
      {
        id: "type_note",
        node_type: "native_tool",
        label: "Type Note in Notepad",
        position: { x: 250, y: 1520 },
        config: {
          tool_name: "app_interact",
          params_json: {
            action: "type_text",
            element_ref: "{{$.nodes.find_editor.element_ref}}",
            text: "{{$.nodes.build_note.result}}",
          },
        },
      },

      // Step 13: Log success
      {
        id: "log_success",
        node_type: "log",
        label: "Log: Success",
        position: { x: 250, y: 1640 },
        config: {
          message_template:
            "Successfully wrote McDonald's application note for {{$.params.location}}",
          level: "info",
        },
      },

      // Step 14: Return result
      {
        id: "output_success",
        node_type: "output",
        label: "Return Summary",
        position: { x: 250, y: 1760 },
        config: {
          result_template:
            "{{$var.summary}}",
        },
      },

      // ── False branch: No results found ────────────────
      {
        id: "log_no_results",
        node_type: "log",
        label: "Log: No Results",
        position: { x: 600, y: 500 },
        config: {
          message_template:
            "No McDonald's job results found for location: {{$.params.location}}",
          level: "warn",
        },
      },
      {
        id: "output_no_results",
        node_type: "output",
        label: "Return Error",
        position: { x: 600, y: 620 },
        config: {
          result_template:
            "No McDonald's job applications found near {{$.params.location}}. Try a different location or search term.",
        },
      },

      // ── Error Handler (catches failures from scrape/LLM/app_interact) ──
      {
        id: "error_handler",
        node_type: "error_handler",
        label: "Handle Errors",
        position: { x: 700, y: 780 },
        config: {
          fallback_value: {
            error: true,
            message:
              "An error occurred while processing the McDonald's application. Check your network connection and try again.",
          },
        },
      },
      {
        id: "output_error",
        node_type: "output",
        label: "Return Error",
        position: { x: 700, y: 920 },
        config: {
          result_template:
            "Error: {{$.nodes.error_handler.message}}",
        },
      },
    ],
    edges: [
      // Trigger → Search
      {
        id: "e_trigger_search",
        source: "trigger_1",
        target: "search_web",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Search → Set Results
      {
        id: "e_search_set",
        source: "search_web",
        target: "set_results",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Set Results → Check
      {
        id: "e_set_check",
        source: "set_results",
        target: "check_results",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Check → True → LLM Pick URL
      {
        id: "e_check_true",
        source: "check_results",
        target: "llm_pick_url",
        source_handle: "true",
        target_handle: null,
        label: null,
      },
      // Check → False → Log No Results
      {
        id: "e_check_false",
        source: "check_results",
        target: "log_no_results",
        source_handle: "false",
        target_handle: null,
        label: null,
      },
      // LLM Pick URL → Scrape
      {
        id: "e_pick_scrape",
        source: "llm_pick_url",
        target: "scrape_page",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Scrape → LLM Summarize
      {
        id: "e_scrape_summarize",
        source: "scrape_page",
        target: "llm_summarize",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // LLM Summarize → Set Summary
      {
        id: "e_summarize_set",
        source: "llm_summarize",
        target: "set_summary",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Set Summary → Build Note
      {
        id: "e_set_build",
        source: "set_summary",
        target: "build_note",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Build Note → Launch Notepad
      {
        id: "e_build_launch",
        source: "build_note",
        target: "launch_notepad",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Launch Notepad → Wait
      {
        id: "e_launch_wait",
        source: "launch_notepad",
        target: "wait_notepad",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Wait → Find Editor
      {
        id: "e_wait_find",
        source: "wait_notepad",
        target: "find_editor",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Find Editor → Type Note
      {
        id: "e_find_type",
        source: "find_editor",
        target: "type_note",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Type Note → Log Success
      {
        id: "e_type_log",
        source: "type_note",
        target: "log_success",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Log Success → Output
      {
        id: "e_log_output",
        source: "log_success",
        target: "output_success",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Log No Results → Output No Results
      {
        id: "e_nolog_output",
        source: "log_no_results",
        target: "output_no_results",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Scrape Page → Error Handler (BFS finds this when scrape/LLM/app nodes fail)
      {
        id: "e_scrape_errorhandler",
        source: "scrape_page",
        target: "error_handler",
        source_handle: null,
        target_handle: null,
        label: null,
      },
      // Error Handler → Error Output
      {
        id: "e_errorhandler_output",
        source: "error_handler",
        target: "output_error",
        source_handle: null,
        target_handle: null,
        label: null,
      },
    ],
    viewport: { x: 0, y: 0, zoom: 0.7 },
  };
}

export const EXAMPLE_FLOWCHARTS: ExampleFlowchart[] = [
  {
    id: "mcdonalds_apply",
    name: "McDonald's Job Application Finder",
    description:
      "Web search + scrape + LLM summary + write to Notepad. Demonstrates native tools, LLM requests, conditions, and app automation.",
    create: createMcdonaldsJobFlow,
  },
];
