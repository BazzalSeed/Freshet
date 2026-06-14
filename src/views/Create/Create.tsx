import { useState } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import { FREE_SOURCES } from "../../bridge/types";
import type { StreamSummary, CadenceMode, DraftResult } from "../../bridge/types";
import "./Create.css";

interface CreateProps {
  onCreated: (s: StreamSummary) => void;
  onCancel: () => void;
}

export function Create({ onCreated, onCancel }: CreateProps) {
  const bridge = useBridge();

  const [topic, setTopic] = useState("");
  const [selectedSources, setSelectedSources] = useState<Set<string>>(new Set());
  const [cadenceMode, setCadenceMode] = useState<CadenceMode>("on_launch");
  const [intervalMinutes, setIntervalMinutes] = useState<number | "">("");
  const [draft, setDraft] = useState<DraftResult | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [creating, setCreating] = useState(false);

  function toggleSource(source: string) {
    setSelectedSources((prev) => {
      const next = new Set(prev);
      if (next.has(source)) {
        next.delete(source);
      } else {
        next.add(source);
      }
      return next;
    });
    // Invalidate draft when sources change
    setDraft(null);
  }

  function handleCadenceModeChange(mode: CadenceMode) {
    setCadenceMode(mode);
    if (mode !== "interval") {
      setIntervalMinutes("");
    }
    setDraft(null);
  }

  const previewDisabled =
    topic.trim() === "" ||
    selectedSources.size === 0 ||
    (cadenceMode === "interval" && (intervalMinutes === "" || Number(intervalMinutes) <= 0));

  const createDisabled = draft === null;

  async function handlePreview() {
    if (previewDisabled) return;
    setPreviewing(true);
    try {
      const result = await bridge.generateFirstDraft({
        topic: topic.trim(),
        sources: Array.from(selectedSources),
        cadence:
          cadenceMode === "interval"
            ? { mode: "interval", intervalMinutes: Number(intervalMinutes) }
            : { mode: cadenceMode },
      });
      setDraft(result);
    } finally {
      setPreviewing(false);
    }
  }

  async function handleCreate() {
    if (!draft) return;
    setCreating(true);
    try {
      const summary = await bridge.createStream(draft.proposedDescription);
      onCreated(summary);
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="create">
      <header className="create-header">
        <h1 className="create-title">New stream</h1>
        <button
          className="create-cancel"
          aria-label="Cancel"
          onClick={onCancel}
          type="button"
        >
          Cancel
        </button>
      </header>

      <div className="create-form">
        {/* Topic */}
        <div className="create-field">
          <label className="create-label" htmlFor="create-topic">
            Topic
          </label>
          <input
            id="create-topic"
            className="create-input"
            aria-label="Topic"
            type="text"
            value={topic}
            onChange={(e) => {
              setTopic(e.target.value);
              setDraft(null);
            }}
            placeholder="What do you want to track?"
          />
        </div>

        {/* Sources */}
        <fieldset className="create-field create-sources-fieldset">
          <legend className="create-label">Sources</legend>
          <div className="create-sources">
            {FREE_SOURCES.map((source) => (
              <label key={source} className="create-source-label">
                <input
                  type="checkbox"
                  checked={selectedSources.has(source)}
                  onChange={() => toggleSource(source)}
                  aria-label={source}
                />
                <span>{source}</span>
              </label>
            ))}
          </div>
        </fieldset>

        {/* Cadence */}
        <div className="create-field">
          <label className="create-label" htmlFor="create-cadence-mode">
            Cadence
          </label>
          <select
            id="create-cadence-mode"
            className="create-select"
            aria-label="Cadence mode"
            value={cadenceMode}
            onChange={(e) => handleCadenceModeChange(e.target.value as CadenceMode)}
          >
            <option value="manual">Manual</option>
            <option value="on_launch">On launch</option>
            <option value="interval">Interval</option>
          </select>

          {cadenceMode === "interval" && (
            <div className="create-interval">
              <label className="create-label-inline" htmlFor="create-interval-minutes">
                Every
              </label>
              <input
                id="create-interval-minutes"
                className="create-input create-input-narrow"
                aria-label="Interval minutes"
                type="number"
                min={1}
                value={intervalMinutes}
                onChange={(e) => {
                  setIntervalMinutes(e.target.value === "" ? "" : Number(e.target.value));
                  setDraft(null);
                }}
                placeholder="60"
              />
              <span className="create-interval-unit">minutes</span>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="create-actions">
          <button
            className="create-preview-btn"
            aria-label="Preview"
            onClick={handlePreview}
            disabled={previewDisabled || previewing}
            type="button"
          >
            {previewing ? "Previewing…" : "Preview"}
          </button>
          <button
            className="create-create-btn"
            aria-label="Create"
            onClick={handleCreate}
            disabled={createDisabled || creating}
            type="button"
          >
            {creating ? "Creating…" : "Create"}
          </button>
        </div>
      </div>

      {/* Draft preview area */}
      {draft && (
        <section
          className="create-preview"
          aria-label="Preview"
          role="region"
        >
          <pre className="create-preview-content">{draft.draftMarkdown}</pre>
        </section>
      )}
    </div>
  );
}
