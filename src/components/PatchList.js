import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';

export default function PatchList({ patches, selected, onSelect }) {
  const [cursor, setCursor] = useState(0);

  useInput((input, key) => {
    if (key.downArrow || input === 'j') {
      setCursor((c) => Math.min(c + 1, patches.length - 1));
    }
    if (key.upArrow || input === 'k') {
      setCursor((c) => Math.max(c - 1, 0));
    }
    if (key.return) {
      onSelect(patches[cursor]);
    }
  });

  if (patches.length === 0) {
    return (
      <Box padding={1} flexDirection="column">
        <Text color="green" bold>✓ No patches applied</Text>
        <Text dimColor> System matches upstream NixOS.</Text>
      </Box>
    );
  }

  const statusColor = (status) => {
    switch (status) {
      case 'applied': return 'green';
      case 'conflict': return 'red';
      case 'pending': return 'yellow';
      case 'merged': return 'blue';
      default: return 'white';
    }
  };

  const statusIcon = (status) => {
    switch (status) {
      case 'applied': return '✓';
      case 'conflict': return '✗';
      case 'pending': return '◌';
      case 'merged': return '◆';
      default: return '·';
    }
  };

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text bold color="cyan">Applied Patches ({patches.length})</Text>
      </Box>
      {patches.map((patch, i) => (
        <Box key={patch.id} flexDirection="column">
          <Box>
            <Text color={i === cursor ? 'cyan' : 'white'} bold={i === cursor}>
              {i === cursor ? '▸ ' : '  '}
            </Text>
            <Text color={statusColor(patch.status)}>
              {statusIcon(patch.status)}
            </Text>
            <Text color={i === cursor ? 'cyan' : 'white'} bold={i === cursor}>
              {' '}{patch.name}
            </Text>
          </Box>
          <Box paddingLeft={4}>
            <Text dimColor>
              {patch.target} · {patch.filesChanged} files · {patch.date}
            </Text>
          </Box>
        </Box>
      ))}
    </Box>
  );
}
