// Phase 5, first bounded slice: a phone/tablet doesn't get a Rust subprocess
// runtime or a Node role — that's a fundamentally different device class
// (per CLUSTER_PLAN.md's own scoping). What it *can* do today, with zero new
// mobile tooling, is open a browser to a URL on the same LAN: this is a real
// HTTP server (axum, actually bound and serving, not a mock) run by the
// Manager, exposing a read-only live view of `ClusterStatus` — the same data
// the desktop Console already shows. This is the "thin observer client":
// a phone becomes a Node/Manager candidate only in some later phase; for now
// it's a legitimate, working way to *see* your Cluster from your pocket.
use crate::cluster_actor::ClusterHandle;
use axum::extract::State;
use axum::response::{Html, Json};
use axum::routing::get;
use axum::Router;
use std::net::{SocketAddr, TcpListener, UdpSocket};

async fn status_handler(State(cluster): State<ClusterHandle>) -> Json<crate::cluster_actor::ClusterStatus> {
    Json(cluster.status().await)
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>BrainBuilder Cluster</title>
<style>
  body { font-family: -apple-system, system-ui, sans-serif; margin: 0; padding: 16px; background: #111; color: #eee; }
  h1 { font-size: 18px; margin-bottom: 4px; }
  .sub { color: #888; font-size: 13px; margin-bottom: 16px; }
  .node { background: #1c1c1c; border-radius: 8px; padding: 10px 12px; margin-bottom: 8px; }
  .node .name { font-weight: 600; }
  .node .meta { color: #999; font-size: 12px; margin-top: 2px; }
  .badge { display: inline-block; background: #2e5fd4; color: white; font-size: 11px; padding: 1px 6px; border-radius: 4px; margin-left: 6px; }
  .empty { color: #888; font-size: 14px; }
</style>
</head>
<body>
  <h1>BrainBuilder Cluster</h1>
  <div class="sub" id="sub">Loading…</div>
  <div id="nodes"></div>
  <script>
    async function refresh() {
      try {
        const res = await fetch('/api/status');
        const s = await res.json();
        document.getElementById('sub').textContent = s.has_cluster
          ? 'Cluster ' + (s.cluster_id || '').slice(0, 12) + '…' + (s.is_manager ? ' (viewing from the Manager)' : '')
          : 'This device is not part of a Cluster yet.';
        const nodesEl = document.getElementById('nodes');
        if (!s.nodes || s.nodes.length === 0) {
          nodesEl.innerHTML = '<div class="empty">No devices yet.</div>';
          return;
        }
        nodesEl.innerHTML = s.nodes.map(n => `
          <div class="node">
            <div class="name">${n.display_name}${n.is_self ? '<span class="badge">MANAGER HOST</span>' : ''}</div>
            <div class="meta">${n.os} · ${n.cpu_cores} cores · ${n.ram_total_bytes ? (n.ram_total_bytes / 1e9).toFixed(1) + ' GB RAM' : 'RAM unknown'}</div>
          </div>
        `).join('');
      } catch (e) {
        document.getElementById('sub').textContent = 'Connection lost — retrying…';
      }
    }
    refresh();
    setInterval(refresh, 2000);
  </script>
</body>
</html>"#;

/// Best-effort real LAN IP: opens a UDP "connection" to a public address
/// (no packet is actually sent for UDP `connect`) purely to ask the OS
/// routing table which local interface/address it would use — the standard
/// portable trick for "what's my LAN IP," since there's no direct syscall
/// for it. Falls back to loopback (still usable from the same machine, just
/// not from a phone) if it fails.
fn detect_lan_ip() -> std::net::IpAddr {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
}

/// Starts the real observer HTTP server and returns the URL a phone/tablet
/// on the same LAN can open to see this Cluster's live status.
pub async fn spawn(cluster: ClusterHandle) -> String {
    let listener = TcpListener::bind("0.0.0.0:0").expect("failed to bind observer HTTP server");
    listener.set_nonblocking(true).expect("failed to set observer listener non-blocking");
    let port = listener.local_addr().expect("bound listener has a local addr").port();
    let lan_ip = detect_lan_ip();

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/status", get(status_handler))
        .with_state(cluster);

    tauri::async_runtime::spawn(async move {
        let listener = tokio::net::TcpListener::from_std(listener).expect("failed to adopt observer listener into tokio");
        if let Err(e) = axum::serve(listener, app).await {
            log::error!("observer HTTP server stopped: {e}");
        }
    });

    let addr = SocketAddr::new(lan_ip, port);
    format!("http://{addr}")
}
