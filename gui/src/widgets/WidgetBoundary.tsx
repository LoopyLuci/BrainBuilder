import { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
  widgetId: string;
  title: string;
  children: ReactNode;
}

interface State {
  error: Error | null;
}

// Per-widget error boundary: the core of the "zero-downtime" story. A widget
// that throws while rendering — including a freshly hot-loaded plugin with a
// bug — is caught here and rendered as a contained error card, so it can never
// take down the whole shell. Resetting lets the user retry after a hot-swap
// replaces the offending widget.
export class WidgetBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // eslint-disable-next-line no-console
    console.error(`[widget:${this.props.widgetId}] crashed:`, error, info.componentStack);
    // Surface it in the console tab too, without importing the store at module
    // scope (avoids a cycle if a widget imports the boundary transitively) —
    // a dynamic import still reaches the same store instance since ES module
    // imports are singletons, it just defers resolution until a crash
    // actually happens instead of at boundary-module load time.
    import('../console/logStore')
      .then(({ logError }) => logError(`"${this.props.title}" crashed: ${error.message}`))
      .catch(() => {});
  }

  reset = () => this.setState({ error: null });

  render() {
    if (this.state.error) {
      return (
        <div className="bb-widget-error" role="alert">
          <div className="bb-text-error" style={{ fontWeight: 600 }}>
            "{this.props.title}" hit an error
          </div>
          <div className="bb-text-muted" style={{ fontSize: 11, whiteSpace: 'pre-wrap' }}>
            {this.state.error.message}
          </div>
          <button className="bb-tabs__tab" onClick={this.reset} style={{ marginTop: 6 }}>
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
