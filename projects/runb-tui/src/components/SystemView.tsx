import React from 'react';
import { Box, Text, useInput } from 'ink';
import { runRunb } from '../utils/runb.js';
import { Header, ShortcutBar, KeyValueList, SectionHeader, theme } from './Shared.js';

export const SystemView: React.FC = () => {
  const [version, setVersion] = React.useState('checking...');
  const [lastAction, setLastAction] = React.useState('');

  React.useEffect(() => {
    setVersion(runRunb('--version'));
  }, []);

  useInput((input) => {
    if (input === 'h') {
      setLastAction(runRunb('--help'));
    }
  });

  return (
    <Box flexDirection="column" flexGrow={1}>
      <SectionHeader title="System Info" />
      <KeyValueList
        labelWidth={12}
        items={[
          { key: '    Version', value: version },
          { key: ' Runtime Root', value: '/run/runb' },
          { key: '    Backends', value: 'diff · git · tar · hardlink' },
        ]}
      />

      {/* Architecture diagram */}
      <Box marginTop={2} flexDirection="column">
        <Text color={theme.muted} bold>Architecture</Text>
        <Box marginTop={1} flexDirection="column" paddingLeft={1}>
          <Text color={theme.brand}>┌─────────────────────────────────────────────────────┐</Text>
          <Text color={theme.brand}>│</Text>
          <Text color={theme.brand}>│</Text>
          <Text color={theme.brand}>│</Text>
          <Text color={theme.brand}>│</Text>
          <Text color={theme.brand}>│</Text>
          <Text color={theme.brand}>└─────────────────────────────────────────────────────┘</Text>
        </Box>
        {/* Overlaid text on the diagram */}
        <Box flexDirection="column" marginTop={-7} paddingLeft={3}>
          <Text color="white" bold>  runb  <Text color={theme.muted}>— chroot-only OCI runtime</Text></Text>
          <Box marginTop={1}>
            <Text color={theme.subtle}>  create → start → stop → delete</Text>
          </Box>
          <Text color={theme.subtle}>  overlay: prepare → teardown → verify → upgrade</Text>
          <Text color={theme.subtle}>  layers: init → commit → list → rebase → bench</Text>
          <Text color={theme.muted}>  Backends: diff | git | tar | hardlink</Text>
        </Box>
      </Box>

      {lastAction && (
        <Box marginTop={2} flexDirection="column">
          <Text color={theme.muted} bold>Output</Text>
          <Box marginTop={1}>
            <Text color={theme.subtle}>{lastAction}</Text>
          </Box>
        </Box>
      )}

      <ShortcutBar
        shortcuts={[
          { key: 'h', label: 'Help' },
        ]}
      />
    </Box>
  );
};
