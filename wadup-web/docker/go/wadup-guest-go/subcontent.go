package wadup

import (
	"encoding/json"
	"fmt"
	"os"
	"sync"
)

var (
	subcontentMu      sync.Mutex
	subcontentCounter int
)

// subContentMetadata represents metadata for bytes emission
type subContentMetadata struct {
	Filename string `json:"filename"`
}

// subContentSliceMetadata represents metadata for slice emission
type subContentSliceMetadata struct {
	Filename string `json:"filename"`
	Offset   int64  `json:"offset"`
	Length   int64  `json:"length"`
}

// EmitBytes emits sub-content bytes for recursive processing.
//
// Writes data to /subcontent/data_N.bin and metadata to /subcontent/metadata_N.json.
// WADUP processes the sub-content when the metadata file is closed.
func EmitBytes(data []byte, filename string) error {
	subcontentMu.Lock()
	n := subcontentCounter
	subcontentCounter++
	subcontentMu.Unlock()

	dataPath := fmt.Sprintf("/subcontent/data_%d.bin", n)
	metadataPath := fmt.Sprintf("/subcontent/metadata_%d.json", n)

	// Write data file first
	dataFile, err := os.Create(dataPath)
	if err != nil {
		return fmt.Errorf("failed to create subcontent data file '%s': %w", dataPath, err)
	}
	if _, err := dataFile.Write(data); err != nil {
		dataFile.Close()
		return fmt.Errorf("failed to write subcontent data file '%s': %w", dataPath, err)
	}
	dataFile.Close()

	// Write metadata file (triggers processing when closed)
	metadata := subContentMetadata{Filename: filename}
	jsonData, err := json.Marshal(metadata)
	if err != nil {
		return fmt.Errorf("failed to serialize subcontent metadata: %w", err)
	}

	metaFile, err := os.Create(metadataPath)
	if err != nil {
		return fmt.Errorf("failed to create subcontent metadata file '%s': %w", metadataPath, err)
	}
	defer metaFile.Close()

	if _, err := metaFile.Write(jsonData); err != nil {
		return fmt.Errorf("failed to write subcontent metadata file '%s': %w", metadataPath, err)
	}

	return nil
}

// EmitSlice emits a slice of the input content as sub-content (zero-copy).
//
// The slice references a range of the original /data.bin content without copying.
// Only writes metadata to /subcontent/metadata_N.json.
func EmitSlice(offset, length int64, filename string) error {
	subcontentMu.Lock()
	n := subcontentCounter
	subcontentCounter++
	subcontentMu.Unlock()

	metadataPath := fmt.Sprintf("/subcontent/metadata_%d.json", n)

	metadata := subContentSliceMetadata{
		Filename: filename,
		Offset:   offset,
		Length:   length,
	}
	jsonData, err := json.Marshal(metadata)
	if err != nil {
		return fmt.Errorf("failed to serialize subcontent slice metadata: %w", err)
	}

	metaFile, err := os.Create(metadataPath)
	if err != nil {
		return fmt.Errorf("failed to create subcontent metadata file '%s': %w", metadataPath, err)
	}
	defer metaFile.Close()

	if _, err := metaFile.Write(jsonData); err != nil {
		return fmt.Errorf("failed to write subcontent metadata file '%s': %w", metadataPath, err)
	}

	return nil
}
