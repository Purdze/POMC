import { useAppStateContext } from "../lib/state"

export default function SettingsPage() {
  const {
    keepOpen,
    setKeepOpen,
  } = useAppStateContext();

  return (
    <div className="page settings-page">
      <h2 className="settings-heading">SETTINGS</h2>

      <div className="settings-section">
        <h3 className="settings-section-title">General</h3>

        <div className="settings-row">
          <div className="settings-row-info">
            <span className="settings-row-label">Language</span>
            <span className="settings-row-desc">
              Display language for the launcher
            </span>
          </div>
          <div className="settings-row-control">
            <button className="settings-select">
              English
            </button>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-info">
            <span className="settings-row-label">
              Keep launcher open
            </span>
            <span className="settings-row-desc">
              Keep the launcher open after the game starts
            </span>
          </div>
          <div className="settings-row-control">
            <button
              className={`settings-toggle ${keepOpen ? "on" : ""}`}
              onClick={() => setKeepOpen(!keepOpen)}
            >
              <div className="settings-toggle-knob" />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
