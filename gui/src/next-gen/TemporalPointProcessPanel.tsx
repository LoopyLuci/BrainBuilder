import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface TemporalForecastView {
  intensity: number;
  confidence: number;
}

export function TemporalPointProcessPanel() {
  const [horizon, setHorizon] = useState('5');
  const [forecast, setForecast] = useState<TemporalForecastView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const runForecast = async () => {
    setBusy(true);
    setError(null);
    setForecast(null);
    try {
      const f = await invoke<TemporalForecastView>('tpp_forecast', { horizon: Number(horizon) });
      setForecast(f);
      logInfo(`tpp_forecast: intensity=${f.intensity.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`tpp_forecast failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Temporal Point Process" subtitle="Event timing and intensity forecasting.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Horizon</div>
        <div className="bb-row">
          <input
            className="bb-input"
            type="number"
            value={horizon}
            onChange={(e) => setHorizon(e.target.value)}
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={runForecast} disabled={busy}>
            Forecast
          </Button>
        </div>
      </div>

      {forecast && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Intensity: </span>
              <span style={{ fontWeight: 600 }}>{forecast.intensity.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Confidence: </span>
              <span style={{ fontWeight: 600 }}>{forecast.confidence.toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
