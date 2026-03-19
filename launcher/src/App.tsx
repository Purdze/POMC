import { useState, useCallback, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  HiHome,
  HiSquares2X2,
  HiNewspaper,
  HiCog6Tooth,
  HiMinus,
  HiSquare2Stack,
  HiXMark,
  HiPlay,
  HiChevronDown,
  HiArrowLeft,
  HiCube,
  HiUserPlus,
  HiTrash,
  HiArrowRightOnRectangle,
  HiPencil,
  HiFolder,
  HiPlus,
  HiDocumentDuplicate,
  HiServer,
  HiUserGroup,
  HiPuzzlePiece,
  HiMagnifyingGlass,
  HiListBullet,
} from "react-icons/hi2";

type Page = "home" | "installations" | "servers" | "friends" | "mods" | "news" | "settings";

interface AuthAccount {
  username: string;
  uuid: string;
  access_token: string;
  expires_at: number;
}

interface Installation {
  id: string;
  name: string;
  version: string;
  lastPlayed: string;
  directory: string;
  width: number;
  height: number;
}

interface GameVersion {
  id: string;
  version_type: string;
}

interface PatchNote {
  title: string;
  version: string;
  date: string;
  summary: string;
  image_url: string;
  entry_type: string;
  content_path: string;
}

const NAV_ITEMS: { id: Page; label: string; icon: React.ReactNode; soon?: boolean }[] = [
  { id: "home", label: "HOME", icon: <HiHome /> },
  { id: "installations", label: "INSTALLATIONS", icon: <HiSquares2X2 /> },
  { id: "servers", label: "SERVERS", icon: <HiServer />, soon: true },
  { id: "friends", label: "FRIENDS", icon: <HiUserGroup />, soon: true },
  { id: "mods", label: "MODS", icon: <HiPuzzlePiece />, soon: true },
  { id: "news", label: "NEWS & UPDATES", icon: <HiNewspaper /> },
];

function App() {
  const [page, setPage] = useState<Page>("home");
  const [accounts, setAccounts] = useState<AuthAccount[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [accountDropdownOpen, setAccountDropdownOpen] = useState(false);
  const [server] = useState("");
  const [keepOpen, setKeepOpen] = useState(true);
  const [modView, setModView] = useState<"list" | "grid">("list");
  const [modSearch, setModSearch] = useState("");
  const [modFilter, setModFilter] = useState("all");
  const [installations, setInstallations] = useState<Installation[]>([
    {
      id: "default",
      name: "Latest Release",
      version: "1.21.11",
      lastPlayed: "Today",
      directory: "default",
      width: 854,
      height: 480,
    },
  ]);
  const [activeInstall, setActiveInstall] = useState("default");
  const [editingInstall, setEditingInstall] = useState<Installation | null>(null);
  const [dialogVersionOpen, setDialogVersionOpen] = useState(false);
  const selectedVersion =
    installations.find((i) => i.id === activeInstall)?.version || "1.21.11";
  const [versions, setVersions] = useState<GameVersion[]>([]);
  const [versionDropdownOpen, setVersionDropdownOpen] = useState(false);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [authLoading, setAuthLoading] = useState(false);
  const [authNotice, setAuthNotice] = useState(false);
  const [status, setStatus] = useState("");
  const [news, setNews] = useState<PatchNote[]>([]);
  const [skinUrl, setSkinUrl] = useState<string | null>(null);

  const account = accounts[activeIndex] || null;
  const username = account?.username || "Steve";
  const [selectedNote, setSelectedNote] = useState<{
    title: string;
    body: string;
  } | null>(null);

  const appWindow = getCurrentWindow();

  const openNote = useCallback(async (note: PatchNote) => {
    try {
      const body = await invoke<string>("get_patch_content", {
        contentPath: note.content_path,
      });
      setSelectedNote({ title: note.title, body });
      setPage("news");
    } catch (e) {
      console.error("Failed to fetch content:", e);
    }
  }, []);

  const minimize = () => { appWindow.minimize(); };
  const toggleMaximize = () => { appWindow.toggleMaximize(); };
  const close = () => { appWindow.close(); };

  const loadSkin = useCallback((uuid: string) => {
    invoke<string>("get_skin_url", { uuid })
      .then(setSkinUrl)
      .catch(() => setSkinUrl(null));
  }, []);

  useEffect(() => {
    invoke<AuthAccount[]>("get_all_accounts").then((accs) => {
      if (accs.length > 0) {
        setAccounts(accs);
        setActiveIndex(0);
        loadSkin(accs[0].uuid);
      }
    });
    invoke<PatchNote[]>("get_patch_notes", { count: 6 })
      .then(setNews)
      .catch((e) => console.error("Failed to fetch news:", e));
    invoke<GameVersion[]>("get_versions", { showSnapshots: false })
      .then(setVersions)
      .catch((e) => console.error("Failed to fetch versions:", e));
  }, [loadSkin]);

  const startAddAccount = useCallback(() => {
    setAccountDropdownOpen(false);
    setAuthNotice(true);
  }, []);

  const confirmAddAccount = useCallback(async () => {
    setAuthNotice(false);
    setAuthLoading(true);
    setStatus("Opening browser - approve the sign-in...");
    try {
      const acc = await invoke<AuthAccount>("add_account");
      setAccounts((prev) => {
        const filtered = prev.filter((a) => a.uuid !== acc.uuid);
        return [...filtered, acc];
      });
      setActiveIndex(
        accounts.filter((a) => a.uuid !== acc.uuid).length
      );
      loadSkin(acc.uuid);
      setStatus(`Signed in as ${acc.username}`);
    } catch (e) {
      setStatus(`Auth failed: ${e}`);
    }
    setAuthLoading(false);
  }, [accounts, loadSkin]);

  const switchAccount = useCallback(
    (index: number) => {
      setActiveIndex(index);
      setAccountDropdownOpen(false);
      if (accounts[index]) {
        loadSkin(accounts[index].uuid);
      }
    },
    [accounts, loadSkin]
  );

  const removeAccount = useCallback(
    (uuid: string) => {
      invoke("remove_account", { uuid });
      setAccounts((prev) => prev.filter((a) => a.uuid !== uuid));
      setActiveIndex(0);
      setAccountDropdownOpen(false);
      setSkinUrl(null);
    },
    []
  );

  const handleLaunch = useCallback(async () => {
    setLaunching(true);
    setStatus("Checking assets...");
    try {
      await invoke("ensure_assets", { version: selectedVersion });
      setStatus("Launching POMC...");
      const result = await invoke<string>("launch_game", {
        uuid: account?.uuid || null,
        server: server || null,
      });
      setStatus(result);
    } catch (e) {
      setStatus(`${e}`);
    }
    setTimeout(() => {
      setLaunching(false);
      setStatus("");
    }, 3000);
  }, [username, server, selectedVersion]);

  return (
    <div className="app">
      <div className="titlebar" data-tauri-drag-region>
        <div className="titlebar-left" data-tauri-drag-region>
          <span className="titlebar-icon"><HiCube /></span>
        </div>
        <span className="titlebar-title" data-tauri-drag-region>
          POMC Launcher
        </span>
        <div className="titlebar-controls">
          <button className="tb-btn" onClick={minimize}>
            <HiMinus />
          </button>
          <button className="tb-btn" onClick={toggleMaximize}>
            <HiSquare2Stack />
          </button>
          <button
            className="tb-btn tb-close"
            onClick={close}
          >
            <HiXMark />
          </button>
        </div>
      </div>

      <div className="layout">
        <nav className="sidebar">
          <div className="sidebar-brand">
            <div className="brand-icon"><HiCube /></div>
            <div className="brand-text">
              <span className="brand-name">POMC</span>
              <span className="brand-sub">LAUNCHER</span>
            </div>
            <span className="brand-version">v0.1.0</span>
          </div>

          <div className="sidebar-nav">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.id}
                className={`nav-btn ${page === item.id ? "active" : ""}`}
                onClick={() => setPage(item.id)}
              >
                <span className="nav-icon">{item.icon}</span>
                <span className="nav-text">{item.label}</span>
                {item.soon && <span className="nav-soon">SOON</span>}
              </button>
            ))}
          </div>

          <div className="sidebar-bottom">
            {account ? (
              <div className="account-switcher">
                {accountDropdownOpen && (
                  <div className="click-away" onClick={() => setAccountDropdownOpen(false)} />
                )}
                <button
                  className="account-bar"
                  onClick={() => setAccountDropdownOpen(!accountDropdownOpen)}
                >
                  <div
                    className="mc-head"
                    style={
                      skinUrl
                        ? { backgroundImage: `url(${skinUrl})` }
                        : undefined
                    }
                  />
                  <span className="account-username">{account.username}</span>
                  <HiChevronDown
                    className={`account-arrow ${accountDropdownOpen ? "open" : ""}`}
                  />
                </button>
                {accountDropdownOpen && (
                  <div className="account-dropdown-menu">
                    {accounts.map((acc, i) => (
                      <div
                        key={acc.uuid}
                        className={`account-option ${i === activeIndex ? "active" : ""}`}
                      >
                        <button
                          className="account-option-btn"
                          onClick={() => switchAccount(i)}
                        >
                          {acc.username}
                        </button>
                        <button
                          className="account-remove"
                          onClick={() => removeAccount(acc.uuid)}
                        >
                          <HiTrash />
                        </button>
                      </div>
                    ))}
                    <button
                      className="account-add"
                      onClick={startAddAccount}
                      disabled={authLoading}
                    >
                      <HiUserPlus />
                      <span>
                        {authLoading ? "Signing in..." : "Add account"}
                      </span>
                    </button>
                    <button
                      className="account-menu-btn"
                      onClick={() => {
                        setPage("settings");
                        setAccountDropdownOpen(false);
                      }}
                    >
                      <HiCog6Tooth />
                      <span>Settings</span>
                    </button>
                    <button
                      className="account-menu-btn logout"
                      onClick={() => {
                        if (account) removeAccount(account.uuid);
                      }}
                    >
                      <HiArrowRightOnRectangle />
                      <span>Log out</span>
                    </button>
                  </div>
                )}
              </div>
            ) : (
              <button
                className="sign-in-sidebar"
                onClick={startAddAccount}
                disabled={authLoading}
              >
                {authLoading ? "Signing in..." : "SIGN IN"}
              </button>
            )}
          </div>
        </nav>

        <main className="content">
          {page === "home" && (
            <div className="page home-page">
              <div className="hero-banner">
                <div className="hero-overlay" />
                <div className="hero-content">
                  <h1 className="hero-title">POMC</h1>
                  <p className="hero-subtitle">
                    RUST-NATIVE MINECRAFT CLIENT
                  </p>
                </div>
              </div>

              <div className="launch-bar">
                <button
                  className={`play-button ${launching ? "launching" : ""}`}
                  onClick={handleLaunch}
                  disabled={launching}
                >
                  <HiPlay className="play-icon" />
                  <span className="play-text">
                    {launching ? "LAUNCHING..." : "PLAY"}
                  </span>
                </button>
              </div>

              <div className="version-badge-wrapper">
                {versionDropdownOpen && (
                  <div className="click-away" onClick={() => setVersionDropdownOpen(false)} />
                )}
                <button
                  className="version-badge"
                  onClick={() => setVersionDropdownOpen(!versionDropdownOpen)}
                >
                  <HiCube className="version-badge-icon" />
                  <span>{selectedVersion}</span>
                  <HiChevronDown
                    className={`version-badge-arrow ${versionDropdownOpen ? "open" : ""}`}
                  />
                </button>
                {versionDropdownOpen && (
                  <div className="version-dropdown">
                    <div className="version-list">
                      {installations.map((inst) => (
                        <button
                          key={inst.id}
                          className={`version-item ${inst.id === activeInstall ? "active" : ""}`}
                          onClick={() => {
                            setActiveInstall(inst.id);
                            setVersionDropdownOpen(false);
                          }}
                        >
                          <span className="version-item-id">{inst.name}</span>
                          <span className="version-item-type">{inst.version}</span>
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {status && <div className="status-toast">{status}</div>}

              <div className="news-section">
                <h2 className="news-heading">LATEST NEWS</h2>
                <div className="news-grid">
                  {news.slice(0, 3).map((item) => (
                    <div
                      className="news-card"
                      key={item.version}
                      onClick={() => openNote(item)}
                    >
                      <div
                        className="news-card-img"
                        style={{ backgroundImage: `url(${item.image_url})` }}
                      >
                        <span className="news-type-badge">
                          {item.entry_type}
                        </span>
                      </div>
                      <div className="news-card-body">
                        <span className="news-date">
                          {item.date.replace(/-/g, ".")}
                        </span>
                        <h3 className="news-title">{item.title}</h3>
                        <p className="news-desc">{item.summary}</p>
                      </div>
                    </div>
                  ))}
                  {news.length === 0 && (
                    <p className="news-loading">Loading patch notes...</p>
                  )}
                </div>
              </div>
            </div>
          )}

          {page === "installations" && (
            <div className="page installs-page">
              <div className="installs-header">
                <h2 className="installs-heading">INSTALLATIONS</h2>
                <button
                  className="installs-new-btn"
                  onClick={() =>
                    setEditingInstall({
                      id: "",
                      name: "",
                      version: "1.21.11",
                      lastPlayed: "Never",
                      directory: "",
                      width: 854,
                      height: 480,
                    })
                  }
                >
                  <HiPlus /> New Installation
                </button>
              </div>

              <div className="installs-list">
                {installations.map((inst) => (
                  <div
                    key={inst.id}
                    className={`install-card ${inst.id === activeInstall ? "active" : ""}`}
                  >
                    <div className="install-card-icon">
                      <HiCube />
                    </div>
                    <div className="install-card-info">
                      <span className="install-card-name">
                        {inst.name}
                      </span>
                      <span className="install-card-version">
                        {inst.version}
                      </span>
                    </div>
                    <span className="install-card-played">
                      {inst.lastPlayed}
                    </span>
                    <button
                      className="install-play-btn"
                      onClick={() => {
                        setActiveInstall(inst.id);
                        setPage("home");
                      }}
                    >
                      <HiPlay /> Play
                    </button>
                    <button
                      className="install-folder-btn"
                      onClick={() => console.log("Open:", inst.directory)}
                    >
                      <HiFolder />
                    </button>
                    <div className="install-card-actions">
                      <button
                        className="install-action-btn"
                        onClick={() => setEditingInstall({ ...inst })}
                        title="Edit"
                      >
                        <HiPencil />
                      </button>
                      <button
                        className="install-action-btn"
                        title="Duplicate"
                        onClick={() => {
                          const dup = {
                            ...inst,
                            id: Date.now().toString(36),
                            name: `${inst.name} (copy)`,
                          };
                          setInstallations((prev) => [...prev, dup]);
                        }}
                      >
                        <HiDocumentDuplicate />
                      </button>
                      {inst.id !== "default" && (
                        <button
                          className="install-action-btn delete"
                          title="Delete"
                          onClick={() => {
                            setInstallations((prev) =>
                              prev.filter((i) => i.id !== inst.id)
                            );
                            if (activeInstall === inst.id) {
                              setActiveInstall("default");
                            }
                          }}
                        >
                          <HiTrash />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {page === "news" && (
            <div className="page news-page">
              {selectedNote ? (
                <div className="note-viewer">
                  <button
                    className="note-back"
                    onClick={() => setSelectedNote(null)}
                  >
                    <HiArrowLeft /> Back
                  </button>
                  <h2 className="note-title">{selectedNote.title}</h2>
                  <div
                    className="note-body"
                    dangerouslySetInnerHTML={{ __html: selectedNote.body }}
                  />
                </div>
              ) : (
                <>
                  <h2 className="news-page-heading">NEWS & UPDATES</h2>
                  <div className="news-grid-full">
                    {news.map((item) => (
                      <div
                        className="news-card-wide"
                        key={item.version}
                        onClick={() => openNote(item)}
                      >
                        <div
                          className="news-card-img-wide"
                          style={{
                            backgroundImage: `url(${item.image_url})`,
                          }}
                        >
                          <span className="news-type-badge">
                            {item.entry_type}
                          </span>
                        </div>
                        <div className="news-card-body-wide">
                          <span className="news-date">
                            {item.date.replace(/-/g, ".")}
                          </span>
                          <h3 className="news-title">{item.title}</h3>
                          <p className="news-desc-full">{item.summary}</p>
                          <span className="news-version">{item.version}</span>
                        </div>
                      </div>
                    ))}
                    {news.length === 0 && (
                      <p className="news-loading">Loading patch notes...</p>
                    )}
                  </div>
                </>
              )}
            </div>
          )}

          {page === "servers" && (
            <div className="page mock-page">
              <div className="mock-banner">This is a preview - functionality coming soon</div>
              <h2 className="mock-heading">SERVERS</h2>
              <div className="mock-list">
                {[
                  { name: "Hypixel", ip: "mc.hypixel.net", players: "48,231", ping: "32ms", online: true },
                  { name: "My SMP", ip: "play.mysmp.com", players: "12", ping: "8ms", online: true },
                  { name: "Mineplex", ip: "us.mineplex.com", players: "3,891", ping: "45ms", online: true },
                  { name: "Local Server", ip: "localhost:25565", players: "1", ping: "1ms", online: false },
                ].map((s) => (
                  <div className="mock-server" key={s.ip}>
                    <div className="mock-server-status">
                      <div className={`mock-dot ${s.online ? "on" : "off"}`} />
                    </div>
                    <div className="mock-server-info">
                      <span className="mock-server-name">{s.name}</span>
                      <span className="mock-server-ip">{s.ip}</span>
                    </div>
                    <span className="mock-server-players">{s.players} players</span>
                    <span className="mock-server-ping">{s.ping}</span>
                    <button className="install-play-btn">
                      <HiPlay /> Join
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {page === "friends" && (
            <div className="page mock-page">
              <div className="mock-banner">This is a preview - functionality coming soon</div>
              <h2 className="mock-heading">FRIENDS</h2>
              <h3 className="mock-subheading">Online - 3</h3>
              <div className="mock-list">
                {[
                  { name: "Friend 1", server: "mc.hypixel.net" },
                  { name: "Friend 2", server: "play.mysmp.com" },
                  { name: "Friend 3", server: "localhost:25565" },
                ].map((f) => (
                  <div className="mock-friend" key={f.name}>
                    <div className="mock-friend-avatar">{f.name.split(" ")[1]}</div>
                    <div className="mock-friend-info">
                      <span className="mock-friend-name">{f.name}</span>
                      <span className="mock-friend-status">{f.server}</span>
                    </div>
                    <button className="mock-join-btn"><HiPlay /> Join</button>
                    <div className="mock-dot on" />
                  </div>
                ))}
              </div>
              <h3 className="mock-subheading">Offline - 4</h3>
              <div className="mock-list">
                {["Friend 4", "Friend 5", "Friend 6", "Friend 7"].map((name) => (
                  <div className="mock-friend" key={name}>
                    <div className="mock-friend-avatar off">{name.split(" ")[1]}</div>
                    <div className="mock-friend-info">
                      <span className="mock-friend-name off">{name}</span>
                      <span className="mock-friend-status">Last seen 2h ago</span>
                    </div>
                    <div className="mock-dot off" />
                  </div>
                ))}
              </div>
            </div>
          )}

          {page === "mods" && (() => {
            const mods = [
              { name: "Mod 1", cat: "performance", desc: "Rendering engine optimization for better frame rates", version: "0.6.1", downloads: "38M", installed: true },
              { name: "Mod 2", cat: "performance", desc: "Dynamic lighting and visual enhancement", version: "1.21.11", downloads: "142M", installed: false },
              { name: "Mod 3", cat: "shaders", desc: "Shader pack loader for post-processing effects", version: "1.8.0", downloads: "25M", installed: false },
              { name: "Mod 4", cat: "utility", desc: "Schematic building tools for pasting and moving structures", version: "0.19.0", downloads: "18M", installed: false },
              { name: "Mod 5", cat: "utility", desc: "Real-time mapping with waypoints and minimap", version: "6.0.0", downloads: "52M", installed: true },
              { name: "Mod 6", cat: "gameplay", desc: "Adds new biomes, creatures, and world generation", version: "2.3.0", downloads: "12M", installed: false },
              { name: "Mod 7", cat: "utility", desc: "Inventory sorting and management tools", version: "1.4.2", downloads: "8M", installed: false },
              { name: "Mod 8", cat: "shaders", desc: "Volumetric clouds and atmospheric effects", version: "3.1.0", downloads: "15M", installed: false },
            ];
            const filtered = mods.filter((m) =>
              (modFilter === "all" || m.cat === modFilter) &&
              m.name.toLowerCase().includes(modSearch.toLowerCase())
            );
            return (
              <div className="page mock-page">
                <div className="mock-banner">This is a preview - functionality coming soon</div>
                <h2 className="mock-heading">MODS</h2>
                <div className="mods-toolbar">
                  <div className="mods-search">
                    <HiMagnifyingGlass className="mods-search-icon" />
                    <input
                      className="mods-search-input"
                      placeholder="Search mods..."
                      value={modSearch}
                      onChange={(e) => setModSearch(e.target.value)}
                    />
                  </div>
                  <div className="mods-filters">
                    {["all", "performance", "shaders", "utility", "gameplay"].map((f) => (
                      <button
                        key={f}
                        className={`mods-filter ${modFilter === f ? "active" : ""}`}
                        onClick={() => setModFilter(f)}
                      >
                        {f.charAt(0).toUpperCase() + f.slice(1)}
                      </button>
                    ))}
                  </div>
                  <div className="mods-view-toggle">
                    <button
                      className={`mods-view-btn ${modView === "list" ? "active" : ""}`}
                      onClick={() => setModView("list")}
                    >
                      <HiListBullet />
                    </button>
                    <button
                      className={`mods-view-btn ${modView === "grid" ? "active" : ""}`}
                      onClick={() => setModView("grid")}
                    >
                      <HiSquares2X2 />
                    </button>
                  </div>
                </div>
                <div className={modView === "grid" ? "mods-grid" : "mock-list"}>
                  {filtered.map((m) => (
                    <div className={modView === "grid" ? "mock-mod-card" : "mock-mod"} key={m.name}>
                      <div className="mock-mod-icon"><HiPuzzlePiece /></div>
                      <div className="mock-mod-info">
                        <span className="mock-mod-name">{m.name}</span>
                        <span className="mock-mod-desc">{m.desc}</span>
                        <div className="mock-mod-meta">
                          <span>{m.version}</span>
                          <span>{m.downloads} downloads</span>
                        </div>
                      </div>
                      <button className={`mock-mod-btn ${m.installed ? "installed" : ""}`}>
                        {m.installed ? "Installed" : "Install"}
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            );
          })()}

          {page === "settings" && (
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
          )}
        </main>
      </div>

      {authNotice && (
        <div className="dialog-overlay" onClick={() => setAuthNotice(false)}>
          <div className="dialog" onClick={(e) => e.stopPropagation()}>
            <h2 className="dialog-title">Sign In Notice</h2>
            <p className="auth-notice-text">
              POMC is currently awaiting approval for direct Microsoft sign-in.
              Until approved, you'll be redirected to enter a code in your
              browser to authenticate. This is a one-time process - your
              session will be saved securely.
            </p>
            <div className="dialog-actions">
              <button
                className="dialog-cancel"
                onClick={() => setAuthNotice(false)}
              >
                Cancel
              </button>
              <button className="dialog-save" onClick={confirmAddAccount}>
                Continue
              </button>
            </div>
          </div>
        </div>
      )}

      {editingInstall && (
        <div className="dialog-overlay" onClick={() => { setEditingInstall(null); setDialogVersionOpen(false); }}>
          <div className="dialog" onClick={(e) => { e.stopPropagation(); if (dialogVersionOpen) setDialogVersionOpen(false); }}>
            <h2 className="dialog-title">
              {editingInstall.id ? "Edit Installation" : "New Installation"}
            </h2>

            <div className="dialog-fields">
              <div className="dialog-field">
                <label>NAME</label>
                <input
                  value={editingInstall.name}
                  onChange={(e) =>
                    setEditingInstall({ ...editingInstall, name: e.target.value })
                  }
                  placeholder="My Installation"
                  autoFocus
                />
              </div>
              <div className="dialog-field">
                <label>VERSION</label>
                <div className="custom-select-wrapper">
                  <button
                    className="custom-select"
                    onClick={() => setDialogVersionOpen(!dialogVersionOpen)}
                    type="button"
                  >
                    <span>{editingInstall.version}</span>
                    <HiChevronDown
                      className={`custom-select-arrow ${dialogVersionOpen ? "open" : ""}`}
                    />
                  </button>
                  {dialogVersionOpen && (
                    <div className="custom-select-dropdown" onClick={(e) => e.stopPropagation()}>
                      <label className="custom-select-toggle">
                        <input
                          type="checkbox"
                          checked={showSnapshots}
                          onChange={(e) => {
                            setShowSnapshots(e.target.checked);
                            invoke<GameVersion[]>("get_versions", {
                              showSnapshots: e.target.checked,
                            }).then(setVersions);
                          }}
                        />
                        <span>Show snapshots</span>
                      </label>
                      <div className="custom-select-list">
                        {versions.map((v) => (
                          <button
                            key={v.id}
                            className={`custom-select-item ${v.id === editingInstall.version ? "active" : ""}`}
                            onClick={() => {
                              setEditingInstall({
                                ...editingInstall,
                                version: v.id,
                              });
                              setDialogVersionOpen(false);
                            }}
                          >
                            <span>{v.id}</span>
                            {v.version_type !== "release" && (
                              <span className="custom-select-tag">
                                {v.version_type}
                              </span>
                            )}
                          </button>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
              <div className="dialog-field">
                <label>GAME DIRECTORY</label>
                <div className="dialog-browse">
                  <input
                    value={editingInstall.directory}
                    onChange={(e) =>
                      setEditingInstall({ ...editingInstall, directory: e.target.value })
                    }
                    placeholder="default"
                  />
                  <button
                    className="dialog-browse-btn"
                    onClick={async () => {
                      const path = await openDialog({ directory: true });
                      if (path) {
                        setEditingInstall({
                          ...editingInstall,
                          directory: path as string,
                        });
                      }
                    }}
                  >
                    <HiFolder />
                  </button>
                </div>
              </div>
              <div className="dialog-field">
                <label>RESOLUTION</label>
                <div className="dialog-resolution">
                  <input
                    type="number"
                    value={editingInstall.width}
                    onChange={(e) =>
                      setEditingInstall({
                        ...editingInstall,
                        width: parseInt(e.target.value) || 854,
                      })
                    }
                    placeholder="854"
                  />
                  <span className="dialog-resolution-x">×</span>
                  <input
                    type="number"
                    value={editingInstall.height}
                    onChange={(e) =>
                      setEditingInstall({
                        ...editingInstall,
                        height: parseInt(e.target.value) || 480,
                      })
                    }
                    placeholder="480"
                  />
                </div>
              </div>
            </div>

            <div className="dialog-actions">
              <button
                className="dialog-cancel"
                onClick={() => setEditingInstall(null)}
              >
                Cancel
              </button>
              <button
                className="dialog-save"
                onClick={async () => {
                  const isNew = !editingInstall.id;
                  const install = {
                    ...editingInstall,
                    id: editingInstall.id || Date.now().toString(36),
                    name: editingInstall.name || "Installation",
                    directory: editingInstall.directory || "default",
                  };
                  setInstallations((prev) => {
                    const filtered = prev.filter((i) => i.id !== install.id);
                    return [...filtered, install];
                  });
                  setEditingInstall(null);
                  if (isNew) {
                    setStatus(`Installing ${install.version}...`);
                    try {
                      await invoke("ensure_assets", { version: install.version });
                      setStatus(`${install.name} ready`);
                    } catch (e) {
                      setStatus(`Install failed: ${e}`);
                    }
                    setTimeout(() => setStatus(""), 3000);
                  }
                }}
              >
                {editingInstall.id ? "Save" : "Install"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
