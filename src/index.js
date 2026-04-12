import React, { useState } from 'react';
import { render, Box, Text, useApp, useInput } from 'ink';
import Header from './components/Header.js';
import PatchList from './components/PatchList.js';
import UpstreamDiff from './components/UpstreamDiff.js';
import DetailPanel from './components/DetailPanel.js';
import StatusBar from './components/StatusBar.js';
import MrWizard from './components/MrWizard.js';
import { useSystemState } from './hooks/useSystemState.js';

function App() {
  const [activeTab, setActiveTab] = useState(0);
  const [selectedItem, setSelectedItem] = useState(null);
  const [showMrWizard, setShowMrWizard] = useState(false);
  const { exit } = useApp();
  const systemState = useSystemState();

  const tabs = ['Patches', 'Upstream Diff', 'Generations'];

  useInput((input, key) => {
    if (showMrWizard) return; // MrWizard handles its own input

    if (input === 'q' || (key.ctrl && input === 'c')) {
      exit();
    }
    if (key.tab) {
      setActiveTab((prev) => (prev + 1) % tabs.length);
    }
    if (key.shift && key.tab) {
      setActiveTab((prev) => (prev - 1 + tabs.length) % tabs.length);
    }
    if (input === 'm') {
      setShowMrWizard(true);
    }
    if (input === 'r') {
      systemState.refresh();
    }
  });

  return (
    <Box flexDirection="column" height="100%">
      <Header activeTab={activeTab} tabs={tabs} />
      <Box flexGrow={1} flexDirection="row">
        <Box width="55%" flexDirection="column" borderStyle="round" borderColor="cyan">
          {activeTab === 0 && (
            <PatchList
              patches={systemState.patches}
              selected={selectedItem}
              onSelect={setSelectedItem}
            />
          )}
          {activeTab === 1 && (
            <UpstreamDiff
              diffs={systemState.upstreamDiffs}
              selected={selectedItem}
              onSelect={setSelectedItem}
            />
          )}
          {activeTab === 2 && (
            <Box padding={1}>
              <Text dimColor>Generation history coming soon...</Text>
            </Box>
          )}
        </Box>
        <Box width="45%" flexDirection="column" borderStyle="round" borderColor="gray">
          <DetailPanel item={selectedItem} tab={activeTab} />
        </Box>
      </Box>
      <StatusBar
        patches={systemState.patches.length}
        diffs={systemState.upstreamDiffs.length}
        loading={systemState.loading}
      />
      {showMrWizard && (
        <MrWizard
          item={selectedItem}
          onClose={() => setShowMrWizard(false)}
        />
      )}
    </Box>
  );
}

render(<App />);
