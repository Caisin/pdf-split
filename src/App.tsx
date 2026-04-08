import * as Tabs from "@radix-ui/react-tabs";
import { useState } from "react";

import "./App.css";
import { TAB_ITEMS, type ToolTab } from "./components/toolTabs";
import { BatchImageWatermarkTool } from "./components/tools/BatchImageWatermarkTool";
import { ExtractImagesTool } from "./components/tools/ExtractImagesTool";
import { PdfWatermarkTool } from "./components/tools/PdfWatermarkTool";
import { SplitPdfTool } from "./components/tools/SplitPdfTool";

function App() {
  const [activeTab, setActiveTab] = useState<ToolTab>("split");

  return (
    <main className="app-shell">
      <section className="tab-shell">
        <Tabs.Root
          className="tabs-root"
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as ToolTab)}
        >
          <Tabs.List className="tab-strip tab-row" aria-label="PDF 工具切换">
            {TAB_ITEMS.map((item) => (
              <Tabs.Trigger className="tab-button" value={item.value} key={item.value}>
                <span className="tab-icon">{item.icon}</span>
                <span className="tab-label">{item.label}</span>
              </Tabs.Trigger>
            ))}
          </Tabs.List>

          <Tabs.Content className="tab-panel" value="split">
            <SplitPdfTool />
          </Tabs.Content>

          <Tabs.Content className="tab-panel" value="extract">
            <ExtractImagesTool />
          </Tabs.Content>

          <Tabs.Content className="tab-panel" value="watermark">
            <PdfWatermarkTool />
          </Tabs.Content>

          <Tabs.Content className="tab-panel" value="imageWatermark">
            <BatchImageWatermarkTool />
          </Tabs.Content>
        </Tabs.Root>
      </section>
    </main>
  );
}

export default App;
