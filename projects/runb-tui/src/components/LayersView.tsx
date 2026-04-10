import React from 'react';
import { Box, Text, useInput } from 'ink';
import {
  ContainerMeta,
  LayerMeta,
  listContainers,
  listLayers,
  formatBytes,
  runRunb,
} from '../utils/runb.js';
import { useSelection } from '../hooks/useSelection.js';
import {
  SplitPane, ListItem, KeyValueList,
  Header, EmptyState, ShortcutBar, SectionHeader,
  theme,
} from './Shared.js';

interface LayerViewProps {
  containerId: string;
}

const LayerList: React.FC<LayerViewProps> = ({ containerId }) => {
  const sel = useSelection<LayerMeta>(
    () => listLayers(containerId),
    5000
  );

  return (
    <Box flexDirection="column">
      <Header title="Layers" count={sel.items.length} />
      {sel.items.length === 0 ? (
        <EmptyState message="No layers" hint="runb init-layer + runb commit" />
      ) : (
        sel.items.map((l, i) => (
          <ListItem
            key={l.layer_number}
            label={`layer-${String(l.layer_number).padStart(3, '0')}  +${l.stats.files_added} -${l.stats.files_deleted} ~${l.stats.files_changed}  ${formatBytes(l.stats.bytes_written)}`}
            selected={i === sel.index}
          />
        ))
      )}
    </Box>
  );
};

const LayerDetail: React.FC<{ layer: LayerMeta | undefined }> = ({ layer }) => {
  if (!layer) return <EmptyState message="Select a layer" />;

  const totalOps = layer.stats.files_added + layer.stats.files_deleted + layer.stats.files_changed;

  return (
    <Box flexDirection="column">
      <SectionHeader title={`Layer ${layer.layer_number}`} />

      {/* Metadata */}
      <KeyValueList
        labelWidth={10}
        items={[
          { key: '  Created', value: new Date(layer.created_at * 1000).toLocaleString('zh-CN') },
          { key: '   Commit', value: layer.description || '(no message)' },
        ]}
      />

      {/* Stats with visual bars */}
      <Box marginTop={1} flexDirection="column">
        <Text color={theme.muted} bold>Changes</Text>
        <Box marginTop={1}>
          <Text color={theme.success}>  +{layer.stats.files_added} added  </Text>
          <Text color={theme.danger}>  -{layer.stats.files_deleted} deleted  </Text>
          <Text color={theme.warning}>  ~{layer.stats.files_changed} changed</Text>
        </Box>
        <Box marginTop={1}>
          <Text color={theme.muted}>  Written: </Text>
          <Text color="white">{formatBytes(layer.stats.bytes_written)}</Text>
          <Text color={theme.muted}>  │ {totalOps} total operations</Text>
        </Box>
      </Box>
    </Box>
  );
};

export const LayersView: React.FC = () => {
  const containers = useSelection<ContainerMeta>(listContainers, 5000);
  const [activeContainer, setActiveContainer] = React.useState<string | null>(null);
  const [lastAction, setLastAction] = React.useState('');

  React.useEffect(() => {
    if (containers.selected && !activeContainer) {
      setActiveContainer(containers.selected.id);
    }
  }, [containers.selected]);

  useInput((input) => {
    if (input === 'c') {
      if (containers.items.length > 0) {
        const idx = containers.items.findIndex(c => c.id === activeContainer);
        const next = containers.items[(idx + 1) % containers.items.length];
        setActiveContainer(next.id);
      }
    }
    if (input === 'i' && activeContainer) {
      setLastAction(runRunb(`init-layer ${activeContainer}`));
    }
    if (input === 'm' && activeContainer) {
      setLastAction(runRunb(`commit ${activeContainer} -m "manual commit"`));
    }
    if (input === 'b' && activeContainer) {
      setLastAction(runRunb(`bench ${activeContainer}`));
    }
  });

  const layerSel = useSelection<LayerMeta>(
    () => activeContainer ? listLayers(activeContainer) : [],
    5000
  );

  const containerList = (
    <Box flexDirection="column">
      <Header title="Containers" count={containers.items.length} />
      {containers.items.map((c, i) => (
        <ListItem
          key={c.id}
          label={`${c.id}${c.id === activeContainer ? ' ◄ active' : ''}`}
          selected={i === containers.index}
          color={c.id === activeContainer ? theme.success : undefined}
        />
      ))}
      {lastAction && (
        <Box marginTop={1}>
          <Text color={theme.success}>✓ {lastAction}</Text>
        </Box>
      )}
    </Box>
  );

  return (
    <Box flexDirection="column" flexGrow={1}>
      <SplitPane
        left={
          <Box flexDirection="column">
            {containerList}
            <Box marginTop={1}>
              <LayerList containerId={activeContainer || ''} />
            </Box>
          </Box>
        }
        right={<LayerDetail layer={layerSel.selected} />}
        leftPercent={0.45}
      />
      <ShortcutBar
        shortcuts={[
          { key: 'c', label: 'Switch' },
          { key: 'i', label: 'Init' },
          { key: 'm', label: 'Commit' },
          { key: 'b', label: 'Bench' },
        ]}
      />
    </Box>
  );
};
