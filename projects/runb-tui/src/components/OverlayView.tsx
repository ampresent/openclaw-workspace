import React from 'react';
import { Box, Text, useInput } from 'ink';
import {
  ContainerMeta,
  OverlayEntry,
  listContainers,
  getOverlayConfig,
  getContainerState,
  runRunb,
} from '../utils/runb.js';
import { useSelection } from '../hooks/useSelection.js';
import {
  SplitPane, ListItem, KeyValueList,
  Header, EmptyState, ShortcutBar, SectionHeader,
  theme,
} from './Shared.js';

export const OverlayView: React.FC = () => {
  const containers = useSelection<ContainerMeta>(listContainers, 5000);
  const [activeContainer, setActiveContainer] = React.useState<string | null>(null);
  const [lastAction, setLastAction] = React.useState('');

  const active = activeContainer || containers.selected?.id;

  const overlays = React.useMemo((): OverlayEntry[] => {
    if (!active) return [];
    const meta = getContainerState(active);
    if (!meta) return [];
    return getOverlayConfig(meta.bundle);
  }, [active]);

  const overlaySel = useSelection<OverlayEntry>(() => overlays, 5000);

  useInput((input) => {
    if (input === 'c' && containers.items.length > 0) {
      const idx = containers.items.findIndex(c => c.id === active);
      const next = containers.items[(idx + 1) % containers.items.length];
      setActiveContainer(next.id);
    }
    if (input === 'p' && active) {
      setLastAction(runRunb(`prepare ${active}`));
    }
    if (input === 't' && active) {
      setLastAction(runRunb(`teardown ${active}`));
    }
    if (input === 'v' && active) {
      setLastAction(runRunb(`verify ${active}`));
    }
  });

  const overlayList = (
    <Box flexDirection="column">
      <Header title="Overlay Mounts" count={overlays.length} />
      {overlays.length === 0 ? (
        <EmptyState
          message="No overlay config"
          hint="Create runb.toml in bundle dir"
        />
      ) : (
        overlays.map((o, i) => (
          <ListItem
            key={i}
            label={`${o.host}  →  ${o.container}`}
            selected={i === overlaySel.index}
            indicator="↦"
            color={theme.accent}
          />
        ))
      )}
      {lastAction && (
        <Box marginTop={1}>
          <Text color={theme.success}>✓ {lastAction}</Text>
        </Box>
      )}
    </Box>
  );

  const detail = overlaySel.selected ? (
    <Box flexDirection="column">
      <SectionHeader title="Overlay Detail" />
      <KeyValueList
        labelWidth={10}
        items={[
          { key: '     Host', value: overlaySel.selected.host, color: 'blue' },
          { key: 'Container', value: overlaySel.selected.container, color: theme.warning },
        ]}
      />

      {/* Config preview */}
      <Box marginTop={2} flexDirection="column">
        <Text color={theme.muted} bold>Config (runb.toml)</Text>
        <Box marginTop={1} flexDirection="column">
          <Text color={theme.muted}>  [overlay]</Text>
          <Text color={theme.muted}>  links = [</Text>
          {overlays.map((o, i) => (
            <Text key={i} color={theme.subtle}>
              {`    { host = "`}
              <Text color="blue">{o.host}</Text>
              {`", container = "`}
              <Text color={theme.warning}>{o.container}</Text>
              {`" }`}
            </Text>
          ))}
          <Text color={theme.muted}>  ]</Text>
        </Box>
      </Box>
    </Box>
  ) : (
    <EmptyState message="Select an overlay entry" />
  );

  return (
    <Box flexDirection="column" flexGrow={1}>
      <SplitPane left={overlayList} right={detail} leftPercent={0.45} />
      <ShortcutBar
        shortcuts={[
          { key: 'c', label: 'Switch' },
          { key: 'p', label: 'Prepare' },
          { key: 't', label: 'Teardown' },
          { key: 'v', label: 'Verify' },
        ]}
      />
    </Box>
  );
};
