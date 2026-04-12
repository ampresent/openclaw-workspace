import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';

export default function StatusBar({ patches, diffs, loading }) {
  return (
    <Box borderStyle="round" borderColor="gray" paddingX={1} justifyContent="space-between">
      <Box gap={2}>
        <Text>
          <Text color="green">● {patches}</Text>
          <Text dimColor> patches</Text>
        </Text>
        <Text>
          <Text color="yellow">● {diffs}</Text>
          <Text dimColor> upstream diffs</Text>
        </Text>
      </Box>
      <Box gap={1}>
        {loading && (
          <>
            <Spinner type="dots" />
            <Text color="yellow">Scanning...</Text>
          </>
        )}
        {!loading && (
          <Text color="green">✓ Ready</Text>
        )}
      </Box>
      <Text dimColor>
        nix-patchwatch v0.1.0
      </Text>
    </Box>
  );
}
