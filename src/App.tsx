import { useState } from "react";
import "./styles.css";
import RecordPage from "./pages/RecordPage";
import HistoryPage from "./pages/HistoryPage";
import QueryPage from "./pages/QueryPage";
import PeoplePage from "./pages/PeoplePage";
import RemindersPage from "./pages/RemindersPage";
import SettingsPage from "./pages/SettingsPage";

const TABS = ["录音", "历史", "回忆", "人脉", "承诺", "设置"] as const;

export default function App() {
  const [tab, setTab] = useState(window.location.hash === "#settings" ? 5 : 0);

  return (
    <div className="app">
      <nav className="tabs">
        {TABS.map((label, i) => (
          <button
            key={label}
            className={i === tab ? "active" : ""}
            onClick={() => setTab(i)}
          >
            {label}
          </button>
        ))}
      </nav>
      <div className="page">
        {tab === 0 && <RecordPage />}
        {tab === 1 && <HistoryPage />}
        {tab === 2 && <QueryPage />}
        {tab === 3 && <PeoplePage />}
        {tab === 4 && <RemindersPage />}
        {tab === 5 && <SettingsPage />}
      </div>
    </div>
  );
}
