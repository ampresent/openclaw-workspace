import React from 'react';
import { Box, Text, useInput } from 'ink';
import {
  ContainerMeta,
  listContainers,
  formatTime,
  stateColor,
  stateIcon,
  runRunb,
} from '../utils/runb.js';
import { useSelection } from '../hooks/useSelection.js';
import {
  SplitPane, ListItem, KeyValueList,
  Header, EmptyState, ShortcutBar, SectionHeader,
  theme,
} from './Shared.js';

export const ContainersView: React.FC = () => {
  const sel = useSelection<ContainerMeta>(listContainers, 3000);

  useInput((input, key) => {
    if (key.upArrow || input === 'k') sel.up();
    if (key.downArrow || input === 'j') sel.down();
    if (input === 's' && sel.selected) {
      runRunb(`start ${sel.selected.id}`);
      setTimeout(sel.refresh, 500);
    }
    if (input === 'k' && sel.selected) {
      runRunb(`stop ${sel.selected.id}`);
      setTimeout(sel.refresh, 500);
    }
    if (input === 'd' && sel.selected) {
      runRunb(`delete ${sel.selected.id}`);
      setTimeout(sel.refresh, 500);
    }
    if (input === 'u' && sel.selected) {
      runRunb(`upgrade ${sel.selected.id}`);
      setTimeout(sel.refresh, 500);
    }
    if (input === 'r') sel.refresh();
  });

  // Count by state
  const running = sel.items.filter(c => c.state === 'running').length;
  const stopped = sel.items.filter(c => c.state === 'stopped').length;

  const containerList = (
    <Box flexDirection="column">
      <Header title="Containers" count={sel.items.length} />
      {sel.items.length === 0 ? (
        <EmptyState
          message="No containers found"
          hint="runb create <id> --bundle <path>"
        />
      ) : (
        <>
          {/* Summary badges */}
          <Box marginBottom={1}>
            <Text color={theme.success}>● {running} running</Text>
            <Text color={theme.muted}>  </Text>
            <Text color={theme.danger}>○ {stopped} stopped</Text>
            <Text color={theme.muted}>  </Text>
            <Text color={theme.muted}>({sel.items.length} total)</Text>
          </Box>
          {sel.items.map((c, i) => (
            <ListItem
              key={c.id}
              label={`${c.id.padEnd(20)} ${c.state}`}
              selected={i === sel.index}
              indicator={stateIcon(c.state)}
              color={stateColor(c.state)}
            />
          ))}
        </>
      )}
    </Box>
  );

  const detail = sel.selected ? (
    <Box flexDirection="column">
      <SectionHeader title={sel.selected.id} />
      <KeyValueList
        labelWidth={8}
        items={[
          { key: 'State', value: `${stateIcon(sel.selected.state)} ${sel.selected.state}`, color: stateColor(sel.selected.state) },
          { key: '    PID', value: sel.selected.pid?.toString() || 'N/A' },
          { key: '  Bundle', value: sel.selected.bundle, color: 'blue' },
          { key: '  Rootfs', value: sel.selected.rootfs, color: 'blue' },
          { key: ' Created', value: formatTime(sel.selected.created_at) },
        ]}
      />
      <Box marginTop={2} flexDirection="column">
        <Text color={theme.muted} bold>Actions</Text>
        <Box marginTop={1}>
          <Text backgroundColor={theme.success} color="black" bold> s </Text>
          <Text> Start  </Text>
          <Text backgroundColor={theme.danger} color="black" bold> k </Text>
          <Text> Stop  </Text>
          <Text backgroundColor={theme.warning} color="black" bold> d </Text>
          <Text> Delete  </Text>
          <Text backgroundColor={theme.accent} color="black" bold> u </Text>
          <Text> Upgrade</Text>
        </Box>
      </Box>
    </Box>
  ) : (
    <EmptyState message="Select a container to view details" />
  );

  return (
    <Box flexDirection="column" flexGrow={1}>
      <SplitPane left={containerList} right={detail} leftPercent={0.45} />
      <ShortcutBar
        shortcuts={[
          { key: 'j/k', label: 'Navigate' },
          { key: 's', label: 'Start' },
          { key: 'k', label: 'Stop' },
          { key: 'd', label: 'Delete' },
          { key: 'u', label: 'Upgrade' },
          { key: 'r', label: 'Refresh' },
        ]}
      />
    </Box>
  );
};
