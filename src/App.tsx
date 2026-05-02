import { useCallback, useEffect, useState } from "react";
import "./styles/theme.css";
import TopBar from "./components/TopBar";
import Sidebar from "./components/Sidebar";
import BlockTerminal from "./components/BlockTerminal";
import RightPanel from "./components/RightPanel";
import StatusBar from "./components/StatusBar";
import SettingsModal from "./components/SettingsModal";
import { api, type OpenRouterModel } from "./lib/api";

export default function App() {
  const [models, setModels] = useState<OpenRouterModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [loadingModels, setLoadingModels] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [hasKey, setHasKey] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string>("idle");

  const fetchModels = useCallback(async () => {
    setLoadingModels(true);
    setStatusMessage("모델 리스트 가져오는 중…");
    try {
      const list = await api.listOpenRouterModels();
      setModels(list);
      if (list.length > 0) {
        setSelectedModel((cur) => cur ?? list[0].id);
      }
      setStatusMessage(`모델 ${list.length}개 로드됨`);
    } catch (e) {
      setStatusMessage(`모델 페치 실패: ${e}`);
    } finally {
      setLoadingModels(false);
    }
  }, []);

  useEffect(() => {
    api
      .hasOpenRouterKey()
      .then((exists) => {
        setHasKey(exists);
        if (exists) {
          fetchModels();
        }
      })
      .catch((e) => setStatusMessage(`초기화 실패: ${e}`));
  }, [fetchModels]);

  const handleSettingsSaved = useCallback(() => {
    setHasKey(true);
    fetchModels();
  }, [fetchModels]);

  return (
    <div className="app">
      <TopBar
        models={models}
        selectedModel={selectedModel}
        onSelectModel={setSelectedModel}
        onOpenSettings={() => setSettingsOpen(true)}
        loading={loadingModels}
      />
      <div className="app__main">
        <Sidebar />
        <BlockTerminal selectedModel={selectedModel} />
        <RightPanel />
      </div>
      <StatusBar
        hasKey={hasKey}
        modelCount={models.length}
        message={statusMessage}
      />
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSaved={handleSettingsSaved}
      />
    </div>
  );
}
