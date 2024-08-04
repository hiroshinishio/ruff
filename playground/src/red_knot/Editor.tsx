import {useCallback, useDeferredValue, useMemo, useState} from "react";
import {Panel, PanelGroup} from "react-resizable-panels";
import {Settings, TargetVersion, Workspace} from "./pkg";
import {ErrorMessage} from "../shared/ErrorMessage";
import Header from "../shared/Header";
import PrimarySideBar from "./PrimarySideBar";
import {HorizontalResizeHandle} from "../shared/ResizeHandle";

import {useTheme} from "../shared/theme";

type Tab = "Source" | "Settings";

interface CheckResult {
  diagnostics: string[];
  error: string | null;
}

type Props = {
  initialSource: string;
  initialSettings: string;
  version: string;
};

export default function Editor({
                                 initialSource,

                                 version,
                               }: Props) {
  const [workspace, setWorkspace] = useState(() => {
    const settings = new Settings();
    settings.target_version = TargetVersion.Py312;
    return new Workspace("/", settings);
  });

  const [revision, setRevision] = useState(0);

  const [file, setFile] = useState(() =>
    workspace.openFile("main.py", initialSource),
  );

  const [tab, setTab] = useState<Tab>("Source");
  const [secondaryTool, setSecondaryTool] = useState<SecondaryTool | null>(
    () => {
      const secondaryValue = new URLSearchParams(location.search).get(
        "secondary",
      );
      if (secondaryValue == null) {
        return null;
      } else {
        return parseSecondaryTool(secondaryValue);
      }
    },
  );

  const [theme, setTheme] = useTheme();

  // Ideally this would be retrieved right from the URL... but routing without a proper
  // router is hard (there's no location changed event) and pulling in a router
  // feels overkill.
  const handleSecondaryToolSelected = (tool: SecondaryTool | null) => {
    if (tool === secondaryTool) {
      tool = null;
    }

    const url = new URL(location.href);

    if (tool == null) {
      url.searchParams.delete("secondary");
    } else {
      url.searchParams.set("secondary", tool);
    }

    history.replaceState(null, "", url);

    setSecondaryTool(tool);
  };

  // TODO: figure out how to do deferred
  const deferredSource = useDeferredValue(file);

  const checkResult: CheckResult = useMemo(() => {
    let file = deferredSource;

    try {
      const diagnostics = workspace.checkFile(file);

      let secondary: SecondaryPanelResult = null;

      try {
        switch (secondaryTool) {
          case "AST":
            secondary = {
              status: "ok",
              content: workspace.parsed(file),
            };
            break;

          case "Format":
            secondary = {
              status: "error",
              content: "Not supported",
            };
            break;

          case "FIR":
            secondary = {
              status: "error",
              content: "Not supported",
            };
            break;

          case "Comments":
            secondary = {
              status: "error",
              content: "Not supported",
            };
            break;

          case "Tokens":
            secondary = {
              status: "ok",
              content: workspace.tokens(file),
            };
            break;
        }
      } catch (error: unknown) {
        secondary = {
          status: "error",
          error: error instanceof Error ? error.message : error + "",
        };
      }

      return {
        diagnostics,
        error: null,
        secondary,
      };
    } catch (e) {
      return {
        diagnostics: [],
        error: (e as Error).message,
        secondary: null,
      };
    }
  }, [deferredSource, secondaryTool]);

  const handleShare = useCallback(() => {
    console.log("TODO");
    // persist(source.settingsSource, source.pythonSource).catch((error) =>
    //   console.error(`Failed to share playground: ${error}`),
    // );
  }, []);

  const handlePythonSourceChange = useCallback((pythonSource: string) => {
    workspace.updateFile(file, pythonSource);
    setRevision((revision) => revision + 1);
  }, []);

  // const handleSettingsSourceChange = useCallback((settingsSource: string) => {
  //   setSource((source) => {
  //     const newSource = {
  //       ...source,
  //       settingsSource,
  //       revision: source.revision + 1,
  //     };
  //
  //     persistLocal(newSource);
  //     return newSource;
  //   });
  // }, []);

  return (
    <main className="flex flex-col h-full bg-ayu-background dark:bg-ayu-background-dark">
      <Header
        edit={revision}
        theme={theme}
        version={version}
        onChangeTheme={setTheme}
        onShare={handleShare}
      />

      <div className="flex flex-grow">
        {
          <PanelGroup direction="horizontal" autoSaveId="main">
            <PrimarySideBar
              onSelectTool={(tool) => setTab(tool)}
              selected={tab}
            />
            <Panel id="main" order={0} className="my-2" minSize={10}>
              <SourceEditor
                visible={tab === "Source"}
                source={source.pythonSource}
                theme={theme}
                diagnostics={checkResult.diagnostics}
                onChange={handlePythonSourceChange}
              />
              {/*<SettingsEditor*/}
              {/*  visible={tab === "Settings"}*/}
              {/*  source={source.settingsSource}*/}
              {/*  theme={theme}*/}
              {/*  onChange={handleSettingsSourceChange}*/}
              {/*/>*/}
            </Panel>
            {secondaryTool != null && (
              <>
                <HorizontalResizeHandle/>
                <Panel
                  id="secondary-panel"
                  order={1}
                  className={"my-2"}
                  minSize={10}
                >
                  {/*<SecondaryPanel*/}
                  {/*  theme={theme}*/}
                  {/*  tool={secondaryTool}*/}
                  {/*  result={checkResult.secondary}*/}
                  {/*/>*/}
                </Panel>
              </>
            )}
            <SecondarySideBar
              selected={secondaryTool}
              onSelected={handleSecondaryToolSelected}
            />
          </PanelGroup>
        }
      </div>
      {checkResult.error && tab === "Source" ? (
        <div
          style={{
            position: "fixed",
            left: "10%",
            right: "10%",
            bottom: "10%",
          }}
        >
          <ErrorMessage>{checkResult.error}</ErrorMessage>
        </div>
      ) : null}
    </main>
  );
}

function parseSecondaryTool(tool: string): SecondaryTool | null {
  if (Object.hasOwn(SecondaryTool, tool)) {
    return tool as any;
  }

  return null;
}
