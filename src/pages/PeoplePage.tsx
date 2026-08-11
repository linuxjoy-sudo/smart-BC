import { useEffect, useState } from "react";
import { api, PersonRow, PreferenceRow } from "../api";

export default function PeoplePage() {
  const [people, setPeople] = useState<PersonRow[]>([]);
  const [prefs, setPrefs] = useState<PreferenceRow[]>([]);
  const [err, setErr] = useState("");

  useEffect(() => {
    api.listPeople().then(setPeople).catch((e) => setErr(String(e)));
    api.listPreferences().then(setPrefs).catch((e) => setErr(String(e)));
  }, []);

  return (
    <div className="people-page">
      <h2>人脉</h2>
      {err && <p className="error">{err}</p>}
      {people.length === 0 && <p className="empty">还没有记住的人。</p>}
      <ul className="list">
        {people.map((p) => (
          <li key={p.id} className="item">
            <strong>{p.name}</strong>
            {p.relation && <span className="tag">{p.relation}</span>}
            {p.note && <p>{p.note}</p>}
          </li>
        ))}
      </ul>
      <h2>偏好</h2>
      {prefs.length === 0 && <p className="empty">还没有偏好记录。</p>}
      <ul className="list">
        {prefs.map((p) => (
          <li key={p.id} className="item">
            <strong>{p.topic}</strong>：{p.value}
          </li>
        ))}
      </ul>
    </div>
  );
}
