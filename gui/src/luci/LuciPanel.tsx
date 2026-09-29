import { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';
import {
  luciStatus,
  luciGreet,
  luciChat,
  luciProposePlan,
  luciListPlans,
  luciUpdatePlanStatus,
  luciReflect,
  luciRecentReflections,
  luciSetPreference,
  luciGetPreference,
  luciRememberFact,
  luciRecallMemories,
  luciForgetMemory,
  luciAudit,
  luciRecentAudit,
  luciRegisterTool,
  luciListTools,
  luciImprove,
  luciRegisterSkill,
  luciListSkills,
  luciObserveAndLearn,
  luciImitateSkill,
  luciDecomposeTask,
  luciRegisterModel,
  luciListModels,
  luciRegisterDataset,
  luciListDatasets,
  luciStartTraining,
  luciListTrainingJobs,
  type LuciChatResponse,
  type LuciPlan,
  type LuciMemory,
  type LuciReflection,
  type LuciTool,
  type LuciAuditEvent,
  type LuciStatusResponse,
  type LuciSkill,
  type LuciTaskCase,
  type LuciModelRecord,
  type LuciDatasetRecord,
  type LuciTrainingJob,
} from '../api/models';

type Tab = 'chat' | 'plans' | 'memories' | 'reflections' | 'tools' | 'audit' | 'skills' | 'observe' | 'models' | 'training';

const MOOD_COLORS: Record<string, string> = {
  happy: '#22c55e',
  excited: '#eab308',
  focused: '#3b82f6',
  reflective: '#a855f7',
  tired: '#94a3b8',
  neutral: '#e2e8f0',
};

export function LuciPanel() {
  const [tab, setTab] = useState<Tab>('chat');
  const [status, setStatus] = useState<LuciStatusResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Chat state
  const [input, setInput] = useState('');
  const [turns, setTurns] = useState<LuciChatResponse[]>([]);
  const [greet, setGreet] = useState<{ greeting: string; mood: string } | null>(null);
  const chatEndRef = useRef<HTMLDivElement | null>(null);

  // Plans state
  const [plans, setPlans] = useState<LuciPlan[]>([]);
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');

  // Memories state
  const [memories, setMemories] = useState<LuciMemory[]>([]);
  const [memoryQuery, setMemoryQuery] = useState('');
  const [rememberText, setRememberText] = useState('');

  // Reflections state
  const [reflections, setReflections] = useState<LuciReflection[]>([]);
  const [reflectText, setReflectText] = useState('');

  // Tools state
  const [tools, setTools] = useState<LuciTool[]>([]);
  const [toolName, setToolName] = useState('');
  const [toolDesc, setToolDesc] = useState('');

  // Audit state
  const [audits, setAudits] = useState<LuciAuditEvent[]>([]);

  // Skills state
  const [skills, setSkills] = useState<LuciSkill[]>([]);
  const [skillName, setSkillName] = useState('');
  const [skillDesc, setSkillDesc] = useState('');
  const [skillImpl, setSkillImpl] = useState('');

  // Observe state
  const [observeTask, setObserveTask] = useState('');
  const [observeText, setObserveText] = useState('');
  const [imitateCase, setImitateCase] = useState<LuciTaskCase | null>(null);

  // Models state
  const [models, setModels] = useState<LuciModelRecord[]>([]);
  const [modelName, setModelName] = useState('');
  const [modelSource, setModelSource] = useState('');
  const [modelUrl, setModelUrl] = useState('');

  // Datasets state
  const [datasets, setDatasets] = useState<LuciDatasetRecord[]>([]);
  const [datasetName, setDatasetName] = useState('');
  const [datasetSource, setDatasetSource] = useState('');

  // Training state
  const [trainingJobs, setTrainingJobs] = useState<LuciTrainingJob[]>([]);
  const [trainingMode, setTrainingMode] = useState('finetune');
  const [selectedModel, setSelectedModel] = useState('');
  const [selectedDatasets, setSelectedDatasets] = useState<string[]>([]);

  useEffect(() => {
    loadStatus();
    loadPlans();
    loadMemories();
    loadReflections();
    loadTools();
    loadAudit();
    loadSkills();
    loadModels();
    loadDatasets();
    loadTrainingJobs();
  }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [turns]);

  const setErr = (e: unknown) => {
    const msg = String(e);
    setError(msg);
    logError(`luci: ${msg}`);
  };

  const loadStatus = async () => {
    try {
      const s = await luciStatus();
      setStatus(s);
    } catch (e) {
      setErr(e);
    }
  };

  const loadPlans = async () => {
    try {
      setPlans(await luciListPlans());
    } catch (e) {
      setErr(e);
    }
  };

  const loadMemories = async () => {
    try {
      setMemories(await luciRecallMemories(memoryQuery || 'general', 20));
    } catch (e) {
      setErr(e);
    }
  };

  const loadReflections = async () => {
    try {
      setReflections(await luciRecentReflections(20));
    } catch (e) {
      setErr(e);
    }
  };

  const loadTools = async () => {
    try {
      setTools(await luciListTools());
    } catch (e) {
      setErr(e);
    }
  };

  const loadAudit = async () => {
    try {
      setAudits(await luciRecentAudit(20));
    } catch (e) {
      setErr(e);
    }
  };

  const loadSkills = async () => {
    try {
      setSkills(await luciListSkills(50));
    } catch (e) {
      setErr(e);
    }
  };

  const loadModels = async () => {
    try {
      setModels(await luciListModels());
    } catch (e) {
      setErr(e);
    }
  };

  const loadDatasets = async () => {
    try {
      setDatasets(await luciListDatasets());
    } catch (e) {
      setErr(e);
    }
  };

  const loadTrainingJobs = async () => {
    try {
      setTrainingJobs(await luciListTrainingJobs());
    } catch (e) {
      setErr(e);
    }
  };

  const handleGreet = async () => {
    setBusy(true);
    try {
      const g = await luciGreet();
      setGreet(g);
      logInfo(`Luci: ${g.greeting}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleSend = async () => {
    const text = input.trim();
    if (!text) return;
    setInput('');
    setBusy(true);
    try {
      const res = await luciChat(text);
      setTurns((prev) => [...prev, res]);
      if (res.plan) {
        loadPlans();
      }
      await luciAudit('user_chat', 'LuciPanel', { text });
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleProposePlan = async () => {
    if (!newTitle.trim()) return;
    setBusy(true);
    try {
      const plan = await luciProposePlan(newTitle.trim(), newDescription.trim(), []);
      setNewTitle('');
      setNewDescription('');
      await loadPlans();
      logInfo(`Plan proposed: ${plan.id}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleRemember = async () => {
    if (!rememberText.trim()) return;
    setBusy(true);
    try {
      await luciRememberFact(rememberText.trim(), ['user']);
      setRememberText('');
      await loadMemories();
      logInfo('Remembered fact');
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleReflect = async () => {
    if (!reflectText.trim()) return;
    setBusy(true);
    try {
      await luciReflect(reflectText.trim());
      setReflectText('');
      await loadReflections();
      logInfo('Reflection saved');
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleRegisterTool = async () => {
    if (!toolName.trim() || !toolDesc.trim()) return;
    setBusy(true);
    try {
      await luciRegisterTool({ name: toolName.trim(), description: toolDesc.trim(), input_schema: {} });
      setToolName('');
      setToolDesc('');
      await loadTools();
      logInfo(`Registered tool: ${toolName.trim()}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleImprove = async () => {
    setBusy(true);
    try {
      const result = await luciImprove();
      logInfo(`Improvement: ${result.status}`);
      await loadStatus();
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleRegisterSkill = async () => {
    if (!skillName.trim() || !skillDesc.trim()) return;
    setBusy(true);
    try {
      await luciRegisterSkill({
        id: crypto.randomUUID(),
        name: skillName.trim(),
        description: skillDesc.trim(),
        tags: [],
        params: {},
        implementation: skillImpl.trim() || 'unknown',
        source: 'user',
        confidence: 0.5,
        success_count: 0,
        failure_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      });
      setSkillName('');
      setSkillDesc('');
      setSkillImpl('');
      await loadSkills();
      logInfo(`Skill registered: ${skillName.trim()}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleObserve = async () => {
    if (!observeTask.trim() || !observeText.trim()) return;
    setBusy(true);
    try {
      const memId = await luciObserveAndLearn(observeTask.trim(), observeText.trim(), { source: 'panel' });
      setObserveTask('');
      setObserveText('');
      logInfo(`Observation stored: ${memId}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleImitate = async () => {
    if (!imitateCase) return;
    setBusy(true);
    try {
      const skill = await luciImitateSkill(imitateCase);
      setImitateCase(null);
      await loadSkills();
      logInfo(`Imitated skill: ${skill.name}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleRegisterModel = async () => {
    if (!modelName.trim() || !modelSource.trim()) return;
    setBusy(true);
    try {
      await luciRegisterModel({
        id: crypto.randomUUID(),
        name: modelName.trim(),
        source: modelSource.trim(),
        model_type: 'unknown',
        format: 'unknown',
        path: undefined,
        url: modelUrl.trim() || undefined,
        metadata: {},
        created_at: new Date().toISOString(),
      });
      setModelName('');
      setModelSource('');
      setModelUrl('');
      await loadModels();
      logInfo(`Model registered: ${modelName.trim()}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleRegisterDataset = async () => {
    if (!datasetName.trim() || !datasetSource.trim()) return;
    setBusy(true);
    try {
      await luciRegisterDataset({
        id: crypto.randomUUID(),
        name: datasetName.trim(),
        source: datasetSource.trim(),
        size: 0,
        format: 'unknown',
        metadata: {},
        created_at: new Date().toISOString(),
      });
      setDatasetName('');
      setDatasetSource('');
      await loadDatasets();
      logInfo(`Dataset registered: ${datasetName.trim()}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const handleStartTraining = async () => {
    if (!selectedModel || selectedDatasets.length === 0) return;
    setBusy(true);
    try {
      const job = await luciStartTraining(selectedModel, trainingMode, selectedDatasets);
      setSelectedModel('');
      setSelectedDatasets([]);
      await loadTrainingJobs();
      logInfo(`Training started: ${job.id}`);
    } catch (e) {
      setErr(e);
    } finally {
      setBusy(false);
    }
  };

  const renderStatusChip = () => {
    if (!status) return null;
    const color = MOOD_COLORS[status.mood] ?? '#e2e8f0';
    return (
      <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
        <span className="bb-chip" style={{ background: `${color}1a`, color, border: `1px solid ${color}55` }}>
          {status.mood}
        </span>
        <span className="bb-text-muted">energy {Math.round(status.energy * 100)}%</span>
        <span className="bb-text-muted">focus: {status.current_focus}</span>
        <span className="bb-text-muted">plans {status.active_plans}</span>
        <span className="bb-text-muted">memories {status.memory_count}</span>
      </div>
    );
  };

  const renderChat = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {greet && (
        <div className="bb-card" style={{ padding: 10 }}>
          <div style={{ fontWeight: 600, color: MOOD_COLORS[greet.mood] ?? '#e2e8f0' }}>{greet.greeting}</div>
          <div className="bb-text-muted" style={{ fontSize: 11 }}>mood: {greet.mood}</div>
        </div>
      )}
      <div style={{ maxHeight: 260, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {turns.length === 0 && <div className="bb-text-muted">Say something friendly to Luci.</div>}
        {turns.map((t, i) => (
          <div key={i} className="bb-card" style={{ padding: 10 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{t.turn.role}</span>
              <span className="bb-text-muted" style={{ fontSize: 11 }}>
                {new Date(t.turn.timestamp).toLocaleTimeString()}
              </span>
            </div>
            <div style={{ marginTop: 4 }}>{t.turn.content}</div>
            <div style={{ marginTop: 4 }}>
              <span className="bb-chip" style={{ color: MOOD_COLORS[t.turn.mood] ?? '#e2e8f0' }}>
                {t.turn.mood}
              </span>
              {t.plan && <span className="bb-chip bb-chip--accent" style={{ marginLeft: 6 }}>plan created</span>}
            </div>
          </div>
        ))}
        <div ref={chatEndRef} />
      </div>
      <div className="bb-row">
        <textarea
          className="bb-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Message Luci…"
          style={{ flex: 1, minHeight: 60 }}
        />
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <Button variant="ghost" onClick={handleGreet} disabled={busy}>
            Greet
          </Button>
          <Button variant="primary" onClick={handleSend} disabled={busy || !input.trim()}>
            Send
          </Button>
        </div>
      </div>
    </div>
  );

  const renderPlans = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">New Plan</div>
        <input className="bb-input" value={newTitle} onChange={(e) => setNewTitle(e.target.value)} placeholder="title" />
        <input
          className="bb-input"
          value={newDescription}
          onChange={(e) => setNewDescription(e.target.value)}
          placeholder="description"
          style={{ marginTop: 6 }}
        />
        <Button variant="primary" onClick={handleProposePlan} disabled={busy || !newTitle.trim()} style={{ marginTop: 8 }}>
          Propose Plan
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {plans.length === 0 && <div className="bb-text-muted">No plans yet.</div>}
        {plans.map((p) => (
          <div key={p.id} className="bb-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{p.title}</span>
              <span className="bb-chip">{p.status}</span>
            </div>
            <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>{p.description}</div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderMemories = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-row">
        <input
          className="bb-input"
          value={memoryQuery}
          onChange={(e) => setMemoryQuery(e.target.value)}
          placeholder="recall query"
          style={{ flex: 1 }}
        />
        <Button variant="ghost" onClick={loadMemories} disabled={busy}>
          Recall
        </Button>
      </div>
      <div className="bb-card">
        <div className="bb-label">Remember Fact</div>
        <textarea
          className="bb-input"
          value={rememberText}
          onChange={(e) => setRememberText(e.target.value)}
          placeholder="Fact to remember..."
          style={{ width: '100%', minHeight: 60 }}
        />
        <Button variant="primary" onClick={handleRemember} disabled={busy || !rememberText.trim()} style={{ marginTop: 8 }}>
          Remember
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {memories.length === 0 && <div className="bb-text-muted">No memories recalled.</div>}
        {memories.map((m) => (
          <div key={m.id} className="bb-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{m.memory_type}</span>
              <span className="bb-text-muted" style={{ fontSize: 11 }}>
                conf {Math.round(m.confidence * 100)}%
              </span>
            </div>
            <div style={{ fontSize: 12 }}>{m.content}</div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderReflections = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">Reflect</div>
        <textarea
          className="bb-input"
          value={reflectText}
          onChange={(e) => setReflectText(e.target.value)}
          placeholder="What did we learn?"
          style={{ width: '100%', minHeight: 70 }}
        />
        <Button variant="primary" onClick={handleReflect} disabled={busy || !reflectText.trim()} style={{ marginTop: 8 }}>
          Save Reflection
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {reflections.length === 0 && <div className="bb-text-muted">No reflections yet.</div>}
        {reflections.map((r) => (
          <div key={r.id} className="bb-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span className="bb-chip">{r.mood}</span>
              <span className="bb-text-muted" style={{ fontSize: 11 }}>
                {new Date(r.created_at).toLocaleString()}
              </span>
            </div>
            <div style={{ fontSize: 12, marginTop: 4 }}>{r.content}</div>
            {r.insights.length > 0 && (
              <ul style={{ marginTop: 6, paddingLeft: 16, fontSize: 12 }}>
                {r.insights.map((x) => (
                  <li key={x}>{x}</li>
                ))}
              </ul>
            )}
          </div>
        ))}
      </div>
    </div>
  );

  const renderTools = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">Register Tool</div>
        <input
          className="bb-input"
          value={toolName}
          onChange={(e) => setToolName(e.target.value)}
          placeholder="tool name"
        />
        <input
          className="bb-input"
          value={toolDesc}
          onChange={(e) => setToolDesc(e.target.value)}
          placeholder="description"
          style={{ marginTop: 6 }}
        />
        <Button variant="primary" onClick={handleRegisterTool} disabled={busy || !toolName.trim() || !toolDesc.trim()} style={{ marginTop: 8 }}>
          Register
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {tools.length === 0 && <div className="bb-text-muted">No tools registered.</div>}
        {tools.map((t) => (
          <div key={t.name} className="bb-card">
            <div style={{ fontWeight: 600 }}>{t.name}</div>
            <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>{t.description}</div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderAudit = () => (
    <div style={{ maxHeight: 260, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
      {audits.length === 0 && <div className="bb-text-muted">No audit events.</div>}
      {audits.map((a) => (
        <div key={a.id} className="bb-card">
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
            <span style={{ fontWeight: 600 }}>{a.event_type}</span>
            <span className="bb-text-muted" style={{ fontSize: 11 }}>
              {new Date(a.created_at).toLocaleTimeString()}
            </span>
          </div>
          <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>source: {a.source}</div>
        </div>
      ))}
    </div>
  );

  const renderSkills = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">Teach Skill</div>
        <input className="bb-input" value={skillName} onChange={(e) => setSkillName(e.target.value)} placeholder="skill name" />
        <input className="bb-input" value={skillDesc} onChange={(e) => setSkillDesc(e.target.value)} placeholder="description" style={{ marginTop: 6 }} />
        <textarea className="bb-input" value={skillImpl} onChange={(e) => setSkillImpl(e.target.value)} placeholder="implementation or source hint" style={{ marginTop: 6, minHeight: 60 }} />
        <Button variant="primary" onClick={handleRegisterSkill} disabled={busy || !skillName.trim() || !skillDesc.trim()} style={{ marginTop: 8 }}>
          Register Skill
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {skills.length === 0 && <div className="bb-text-muted">No skills registered.</div>}
        {skills.map((s) => (
          <div key={s.id} className="bb-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{s.name}</span>
              <span className="bb-text-muted" style={{ fontSize: 11 }}>conf {Math.round(s.confidence * 100)}%</span>
            </div>
            <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>{s.description}</div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderObserve = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">Observe and Learn</div>
        <input className="bb-input" value={observeTask} onChange={(e) => setObserveTask(e.target.value)} placeholder="task/context" />
        <textarea className="bb-input" value={observeText} onChange={(e) => setObserveText(e.target.value)} placeholder="What Luci should learn from..." style={{ marginTop: 6, minHeight: 70 }} />
        <Button variant="primary" onClick={handleObserve} disabled={busy || !observeTask.trim() || !observeText.trim()} style={{ marginTop: 8 }}>
          Store Observation
        </Button>
      </div>
      <div className="bb-card">
        <div className="bb-label">Imitate from Example</div>
        <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>
          Use an existing task case to derive a new skill by imitation.
        </div>
      </div>
    </div>
  );

  const renderModels = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">Register Model</div>
        <input className="bb-input" value={modelName} onChange={(e) => setModelName(e.target.value)} placeholder="model name" />
        <input className="bb-input" value={modelSource} onChange={(e) => setModelSource(e.target.value)} placeholder="local path or registry id" style={{ marginTop: 6 }} />
        <input className="bb-input" value={modelUrl} onChange={(e) => setModelUrl(e.target.value)} placeholder="optional remote url" style={{ marginTop: 6 }} />
        <Button variant="primary" onClick={handleRegisterModel} disabled={busy || !modelName.trim() || !modelSource.trim()} style={{ marginTop: 8 }}>
          Register Model
        </Button>
      </div>
      <div className="bb-card">
        <div className="bb-label">Register Dataset</div>
        <input className="bb-input" value={datasetName} onChange={(e) => setDatasetName(e.target.value)} placeholder="dataset name" />
        <input className="bb-input" value={datasetSource} onChange={(e) => setDatasetSource(e.target.value)} placeholder="local path or url" style={{ marginTop: 6 }} />
        <Button variant="primary" onClick={handleRegisterDataset} disabled={busy || !datasetName.trim() || !datasetSource.trim()} style={{ marginTop: 8 }}>
          Register Dataset
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {models.length === 0 && <div className="bb-text-muted">No models registered.</div>}
        {models.map((m) => (
          <div key={m.id} className="bb-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{m.name}</span>
              <span className="bb-text-muted" style={{ fontSize: 11 }}>{m.format}</span>
            </div>
            <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>{m.source}</div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderTraining = () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div className="bb-card">
        <div className="bb-label">Start Training</div>
        <select className="bb-input" value={selectedModel} onChange={(e) => setSelectedModel(e.target.value)}>
          <option value="">Select model</option>
          {models.map((m) => (
            <option key={m.id} value={m.id}>{m.name}</option>
          ))}
        </select>
        <input className="bb-input" value={trainingMode} onChange={(e) => setTrainingMode(e.target.value)} placeholder="mode: finetune, lora, self_train" style={{ marginTop: 6 }} />
        <div style={{ marginTop: 6, fontSize: 12, color: 'rgb(148,163,184)' }}>
          Datasets: {datasets.map((d) => d.id).join(', ')}
        </div>
        <Button variant="primary" onClick={handleStartTraining} disabled={busy || !selectedModel || datasets.length === 0} style={{ marginTop: 8 }}>
          Start Training
        </Button>
      </div>
      <div style={{ maxHeight: 220, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {trainingJobs.length === 0 && <div className="bb-text-muted">No training jobs.</div>}
        {trainingJobs.map((j) => (
          <div key={j.id} className="bb-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{j.mode}</span>
              <span className="bb-chip">{j.status}</span>
            </div>
            <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>model: {j.model_id}</div>
          </div>
        ))}
      </div>
    </div>
  );

  return (
    <Panel
      title="Luci"
      subtitle="Personal companion assistant for model design and development."
      action={
        <div style={{ display: 'flex', gap: 6 }}>
          <Button variant="ghost" onClick={loadStatus} disabled={busy}>
            Refresh
          </Button>
          <Button variant="primary" onClick={handleImprove} disabled={busy}>
            Improve
          </Button>
        </div>
      }
    >
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}

      <div style={{ marginBottom: 10 }}>{renderStatusChip()}</div>

      <div
        style={{
          display: 'flex',
          gap: 6,
          marginBottom: 10,
          borderBottom: '1px solid rgba(255,255,255,0.08)',
          paddingBottom: 6,
          overflowX: 'auto',
        }}
      >
        {([
          ['chat', 'Chat'],
          ['plans', 'Plans'],
          ['memories', 'Memories'],
          ['reflections', 'Reflections'],
          ['tools', 'Tools'],
          ['audit', 'Audit'],
          ['skills', 'Skills'],
          ['observe', 'Observe'],
          ['models', 'Models'],
          ['training', 'Training'],
        ] as const).map(([key, label]) => (
          <button
            key={key}
            onClick={() => setTab(key)}
            className="bb-chip"
            style={{
              cursor: 'pointer',
              background: tab === key ? 'rgba(59,130,246,0.25)' : undefined,
              color: tab === key ? '#e2e8f0' : undefined,
              border: `1px solid ${tab === key ? 'rgba(59,130,246,0.55)' : 'rgba(255,255,255,0.08)'}`,
            }}
          >
            {label}
          </button>
        ))}
      </div>

      {tab === 'chat' && renderChat()}
      {tab === 'plans' && renderPlans()}
      {tab === 'memories' && renderMemories()}
      {tab === 'reflections' && renderReflections()}
      {tab === 'tools' && renderTools()}
      {tab === 'audit' && renderAudit()}
      {tab === 'skills' && renderSkills()}
      {tab === 'observe' && renderObserve()}
      {tab === 'models' && renderModels()}
      {tab === 'training' && renderTraining()}
    </Panel>
  );
}
