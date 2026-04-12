import React from 'react';
import { Box, Text } from 'ink';

export default function DetailPanel({ item, tab }) {
  if (!item) {
    return (
      <Box padding={1}>
        <Text dimColor>
          {tab === 0 && 'Select a patch to view details'}
          {tab === 1 && 'Select a diff to view changes'}
          {tab === 2 && 'Select a generation to compare'}
        </Text>
      </Box>
    );
  }

  if (tab === 0) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold color="cyan" underline>{item.name}</Text>
        <Box marginTop={1} flexDirection="column" gap={0}>
          <Row label="Target" value={item.target} />
          <Row label="Status" value={item.status} color={item.status === 'applied' ? 'green' : 'yellow'} />
          <Row label="Author" value={item.author || 'NixOS config'} />
          <Row label="Date" value={item.date} />
          <Row label="Files" value={`${item.filesChanged} changed`} />
          <Row label="Source" value={item.source || 'local'} />
        </Box>
        {item.description && (
          <Box marginTop={1} flexDirection="column">
            <Text bold>Description</Text>
            <Box paddingLeft={2} borderStyle="single" borderColor="gray" marginTop={0}>
              <Text>{item.description}</Text>
            </Box>
          </Box>
        )}
        {item.diff && (
          <Box marginTop={1} flexDirection="column">
            <Text bold>Diff Preview</Text>
            <DiffView diff={item.diff} />
          </Box>
        )}
      </Box>
    );
  }

  if (tab === 1) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold color="yellow" underline>{item.path}</Text>
        <Box marginTop={1} flexDirection="column" gap={0}>
          <Row label="Package" value={item.package} />
          <Row label="Type" value={item.type} color={item.type === 'added' ? 'green' : item.type === 'removed' ? 'red' : 'yellow'} />
          <Row label="Added" value={`+${item.addedLines} lines`} color="green" />
          <Row label="Removed" value={`-${item.removedLines} lines`} color="red" />
        </Box>
        {item.hunks && (
          <Box marginTop={1} flexDirection="column">
            <Text bold>Changes</Text>
            <DiffView diff={item.hunks} />
          </Box>
        )}
        <Box marginTop={1}>
          <Text dimColor>Press [M] to submit merge request for this file</Text>
        </Box>
      </Box>
    );
  }

  return null;
}

function Row({ label, value, color }) {
  return (
    <Box>
      <Box width={12}><Text dimColor>{label}:</Text></Box>
      <Text color={color || 'white'}>{value}</Text>
    </Box>
  );
}

function DiffView({ diff }) {
  const lines = diff.split('\n').slice(0, 20);
  return (
    <Box flexDirection="column" borderStyle="single" borderColor="gray" marginTop={0} paddingX={1}>
      {lines.map((line, i) => {
        let color = 'white';
        if (line.startsWith('+') && !line.startsWith('+++')) color = 'green';
        else if (line.startsWith('-') && !line.startsWith('---')) color = 'red';
        else if (line.startsWith('@@')) color = 'cyan';
        else if (line.startsWith('diff') || line.startsWith('index')) color = 'yellow';
        return (
          <Text key={i} color={color}>{line}</Text>
        );
      })}
      {diff.split('\n').length > 20 && (
        <Text dimColor>... ({diff.split('\n').length - 20} more lines)</Text>
      )}
    </Box>
  );
}
