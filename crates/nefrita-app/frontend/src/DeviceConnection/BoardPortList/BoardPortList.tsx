import { useCallback, useEffect, useState } from "react";
import type { BoardPort } from "../../bindings/BoardPort";
import type { BoardPortSearchError } from "../../bindings/BoardPortSearchError";
import { searchBoardPorts } from "../../commands/boardPorts";
import { commandFailureMessage } from "../commandFailureMessage";

type BoardPortSearch =
  | { stage: "searching" }
  | { stage: "found"; boardPorts: BoardPort[] }
  | { stage: "failed"; failureMessage: string };

const searchErrorMessages: Record<BoardPortSearchError, string> = {
  "port-enumeration-failed": "The system could not list its serial ports.",
  "search-interrupted": "The board search stopped before it finished.",
};

const unknownSearchFailureMessage = "The board search failed.";

export function BoardPortList({ onConnect }: { onConnect: (portName: string) => void }) {
  const [boardPortSearch, setBoardPortSearch] = useState<BoardPortSearch>({ stage: "searching" });

  const startBoardPortSearch = useCallback(async () => {
    setBoardPortSearch({ stage: "searching" });
    try {
      const boardPorts = await searchBoardPorts();
      setBoardPortSearch({ stage: "found", boardPorts });
    } catch (searchFailure) {
      setBoardPortSearch({
        stage: "failed",
        failureMessage: commandFailureMessage(
          searchFailure,
          searchErrorMessages,
          unknownSearchFailureMessage,
        ),
      });
    }
  }, []);

  useEffect(() => {
    startBoardPortSearch();
  }, [startBoardPortSearch]);

  return (
    <section>
      <h2>Plugged-in boards</h2>
      <BoardPortSearchOutcome boardPortSearch={boardPortSearch} onConnect={onConnect} />
      <button
        type="button"
        onClick={startBoardPortSearch}
        disabled={boardPortSearch.stage === "searching"}
      >
        Search again
      </button>
    </section>
  );
}

function BoardPortSearchOutcome({
  boardPortSearch,
  onConnect,
}: {
  boardPortSearch: BoardPortSearch;
  onConnect: (portName: string) => void;
}) {
  switch (boardPortSearch.stage) {
    case "searching":
      return <p>Searching for boards…</p>;
    case "failed":
      return <p role="alert">{boardPortSearch.failureMessage}</p>;
    case "found":
      if (boardPortSearch.boardPorts.length === 0) {
        return <p>No board found. Plug one in with a USB cable that carries data.</p>;
      }
      return (
        <ul>
          {boardPortSearch.boardPorts.map((boardPort) => (
            <li key={boardPort.portName}>
              <strong>{boardPort.portName}</strong>
              <span>{boardPort.chipName}</span>
              {boardPort.productName !== null && <span>{boardPort.productName}</span>}
              {boardPort.serialNumber !== null && (
                <span>Serial number {boardPort.serialNumber}</span>
              )}
              <button
                type="button"
                aria-label={`Connect ${boardPort.portName}`}
                onClick={() => onConnect(boardPort.portName)}
              >
                Connect
              </button>
            </li>
          ))}
        </ul>
      );
  }
}
