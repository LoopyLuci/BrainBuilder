use tauri::{command, Manager, State, Window};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsEvalResult {
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomQueryResult {
    pub selector: String,
    pub count: usize,
    pub html: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub tag: String,
    pub id: Option<String>,
    pub class: Option<String>,
    pub text: Option<String>,
    pub href: Option<String>,
    pub xpath: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSnapshot {
    pub url: String,
    pub title: String,
    pub elements: Vec<UiElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugState {
    pub enabled: bool,
    pub log_level: String,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            enabled: cfg!(debug_assertions),
            log_level: "info".to_string(),
        }
    }
}

#[command]
pub async fn webview_debug_eval(
    window: Window,
    js: String,
) -> Result<JsEvalResult, String> {
    let script = format!(
        r#"
    (function() {{
      try {{
        const result = (function() {{ {js} }})();
        return JSON.stringify({{ success: true, result: result ?? null }});
      }} catch (e) {{
        return JSON.stringify({{ success: false, error: e.message }});
      }}
    }})()
    "#
    );
    
    let serialized = match window.eval(&script) {
        Ok(val) => serde_json::to_string(&val).map_err(|e| format!("Failed to serialize eval result: {}", e))?,
        Err(e) => return Ok(JsEvalResult {
            success: false,
            result: None,
            error: Some(e.to_string()),
        }),
    };
    
    if let Ok(parsed) = serde_json::from_str::<JsEvalResult>(&serialized) {
        Ok(parsed)
    } else {
        Ok(JsEvalResult {
            success: true,
            result: Some(serialized),
            error: None,
        })
    }
}

#[command]
pub async fn webview_debug_query(
    window: Window,
    selector: String,
) -> Result<DomQueryResult, String> {
    let js = format!(
        r#"
    (function() {{
      const elements = document.querySelectorAll('{selector}');
      const count = elements.length;
      let html = null;
      let text = null;
      if (count > 0) {{
        html = elements[0].outerHTML;
        text = elements[0].textContent?.trim() ?? null;
      }}
      return JSON.stringify({{ selector: '{selector}', count, html, text }});
    }})()
    "#
    );
    
    let serialized = match window.eval(&js) {
        Ok(val) => serde_json::to_string(&val).map_err(|e| format!("Failed to serialize query result: {}", e))?,
        Err(e) => return Err(e.to_string()),
    };
    
    serde_json::from_str(&serialized)
        .map_err(|e| format!("Failed to parse query result: {}", e))
}

#[command]
pub async fn webview_debug_snapshot(
    window: Window,
) -> Result<UiSnapshot, String> {
    let js = r#"
    (function() {
      const elements = [];
      const walker = document.createTreeWalker(
        document.body,
        NodeFilter.SHOW_ELEMENT,
        { acceptNode: function(node) {
            // Skip hidden elements
            const style = window.getComputedStyle(node);
            if (style.display === 'none' || style.visibility === 'hidden') {
              return NodeFilter.FILTER_REJECT;
            }
            return NodeFilter.FILTER_ACCEPT;
          }
        }
      );
      
      let node;
      let count = 0;
      const maxElements = 100;
      
      while (count < maxElements && (node = walker.nextNode())) {
        const el = node;
        const xpath = getXPath(el);
        if (xpath) {
          elements.push({
            tag: el.tagName.toLowerCase(),
            id: el.id || null,
            class: el.className || null,
            text: (el.textContent || '').trim().substring(0, 100) || null,
            href: el.href || null,
            xpath: xpath
          });
          count++;
        }
      }
      
      return JSON.stringify({
        url: window.location.href,
        title: document.title,
        elements: elements
      });
      
      function getXPath(element) {
        if (element.id) {
          return '//*[@id="' + element.id + '"]';
        }
        if (element === document.body) {
          return '/html/body';
        }
        let ix = 0;
        const siblings = element.parentNode?.childNodes || [];
        for (let i = 0; i < siblings.length; i++) {
          const sibling = siblings[i];
          if (sibling === element) {
            return getXPath(element.parentNode) + '/' + element.tagName.toLowerCase() + '[' + (ix + 1) + ']';
          }
          if (sibling.nodeType === 1 && sibling.tagName === element.tagName) {
            ix++;
          }
        }
        return null;
      }
    })()
    "#;
    
    let serialized = match window.eval(js) {
        Ok(val) => serde_json::to_string(&val).map_err(|e| format!("Failed to serialize snapshot: {}", e))?,
        Err(e) => return Err(e.to_string()),
    };
    
    serde_json::from_str(&serialized)
        .map_err(|e| format!("Failed to parse snapshot: {}", e))
}

#[command]
pub async fn webview_debug_click(
    window: Window,
    selector: String,
) -> Result<JsEvalResult, String> {
    let js = format!(
        r#"
    (function() {{
      const el = document.querySelector('{selector}');
      if (!el) {{
        return JSON.stringify({{ success: false, error: 'Element not found: {selector}' }});
      }}
      el.click();
      return JSON.stringify({{ success: true, result: 'clicked' }});
    }})()
    "#
    );
    
    webview_debug_eval(window, js).await
}

#[command]
pub async fn webview_debug_fill(
    window: Window,
    selector: String,
    value: String,
) -> Result<JsEvalResult, String> {
    let js = format!(
        r#"
    (function() {{
      const el = document.querySelector('{selector}');
      if (!el) {{
        return JSON.stringify({{ success: false, error: 'Element not found: {selector}' }});
      }}
      const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      nativeInputValueSetter.call(el, '{value}');
      el.dispatchEvent(new Event('input', {{ bubbles: true }}));
      el.dispatchEvent(new Event('change', {{ bubbles: true }}));
      return JSON.stringify({{ success: true, result: 'filled' }});
    }})()
    "#
    );
    
    webview_debug_eval(window, js).await
}

#[command]
pub async fn webview_debug_get_state(
    window: Window,
) -> Result<UiSnapshot, String> {
    webview_debug_snapshot(window).await
}

#[command]
pub async fn webview_debug_set_enabled(
    state: State<'_, Arc<Mutex<DebugState>>>,
    enabled: bool,
) -> Result<(), String> {
    let mut debug_state = state.lock().await;
    debug_state.enabled = enabled;
    Ok(())
}

#[command]
pub async fn webview_debug_is_enabled(
    state: State<'_, Arc<Mutex<DebugState>>>,
) -> Result<bool, String> {
    let debug_state = state.lock().await;
    Ok(debug_state.enabled)
}
