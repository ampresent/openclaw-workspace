import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';

export default function UpstreamDiff({ diffs, selected, onSelect }) {
  const [cursor, setCursor] = useState(0);

  useInput((input, key) => {
    if (key.downArrow || input === 'j') {
      setCursor((c) => Math.min(c + 1, diffs.length - 1));
    }
    if (key.upArrow || input === 'k') {
      setCursor((c) => Math.max(c - 1, 0));
    }
    if (key.return) {
      onSelect(diffs[cursor]);
    }
  });

  if (diffs.length === 0) {
    return (
      <Box padding={1} flexDirection="column">
        <Text color="green" bold>✓ Fully synced with upstream</Text>
        <Text dimColor> No differences detected.</Text>
      </Box>
    );
  }

  const typeColor = (type) => {
    switch (type) {
      case 'added': return 'green';
      case 'removed': return 'red';
      case 'modified': return 'yellow';
      case 'renamed': return 'blue';
      default: return 'white';
    }
  };

  const typeIcon = (type) => {
    switch (type) {
      case 'added': return '+';
      case 'removed': return '-';
      case 'modified': return '~';
      case 'renamed': return '→';
      default: return '·';
    }
  };

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text bold color="yellow">Upstream Differences ({diffs.length})</Text>
      </Box>
      {diffs.map((diff, i) => (
        <Box key={diff.path} flexDirection="column">
          <Box>
            <Text color={i === cursor ? 'cyan' : 'white'} bold={i === cursor}>
              {i === cursor ? '▸ ' : '  '}
            </Text>
            <Text color={typeColor(diff.type)} bold>
              {typeIcon(diff.type)}
            </Text>
            <Text color={i === cursor ? 'cyan' : 'white'} bold={i === cursor}>
              {' '}{diff.path}
            </Text>
          </Box>
          <Box paddingLeft={4}>
            <Text dimColor>
              {diff.package} · +{diff.addedLines}/-{diff.removedLines} lines
            </Text>
          </Box>
        </Box>
      ))}
    </Box>
  );
}
