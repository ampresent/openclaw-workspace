import React from 'react';
import { Box, Text, useApp, useInput, useStdout } from 'ink';
import { TabBar, useTerminalWidth, theme, Separator } from './components/Shared.js';
import { ContainersView } from './components/ContainersView.js';
import { LayersView } from './components/LayersView.js';
import { OverlayView } from './components/OverlayView.js';
import { SystemView } from './components/SystemView.js';

const TABS = ['Containers', 'Layers', 'Overlay', 'System'];

const App: React.FC = () => {
  const { exit } = useApp();
  const [tab, setTab] = React.useState(0);
  const width = useTerminalWidth();

  useInput((input, key) => {
    if (key.ctrl && (input === 'c' || input === 'q')) exit();
  });

  const renderView = () => {
    switch (tab) {
      case 0: return <ContainersView />;
      case 1: return <LayersView />;
      case 2: return <OverlayView />;
      case 3: return <SystemView />;
      default: return <ContainersView />;
    }
  };

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      {/* ── Title Bar ── */}
      <Box marginBottom={1}>
        <Text color={theme.brand} bold>⬢ </Text>
        <Text color="white" bold>runb</Text>
        <Text color={theme.muted}> ─ Lightweight OCI Container Runtime</Text>
      </Box>

      {/* ── Tab Bar ── */}
      <TabBar tabs={TABS} active={tab} onChange={setTab} />

      {/* ── Content Area ── */}
      <Box flexDirection="column" flexGrow={1} marginTop={1}>
        {renderView()}
      </Box>

      {/* ── Bottom Bar ── */}
      <Separator />
      <Box marginTop={1}>
        <Box marginRight={3}>
          <Text backgroundColor={theme.muted} color="black" bold> Tab </Text>
          <Text color={theme.subtle}> Next</Text>
        </Box>
        <Box marginRight={3}>
          <Text backgroundColor={theme.muted} color="black" bold> 1-4 </Text>
          <Text color={theme.subtle}> Tab</Text>
        </Box>
        <Box marginRight={3}>
          <Text backgroundColor={theme.muted} color="black" bold> ^Q  </Text>
          <Text color={theme.subtle}> Quit</Text>
        </Box>
        <Box flexGrow={1} justifyContent="flex-end">
          <Text color={theme.muted} dimColor>runb-tui v0.1.0</Text>
        </Box>
      </Box>
    </Box>
  );
};

export default App;
