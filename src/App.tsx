import { useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { AlertCircleIcon, CheckCircle2Icon, EraserIcon } from "lucide-react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Spinner } from "@/components/ui/spinner"

type PatchResult = {
  ok: boolean
  message: string
  rva: string | null
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

export function App() {
  const [logs, setLogs] = useState<string[]>([
    "Ready. Click Remove watermark to patch Explorer.",
  ])
  const [running, setRunning] = useState(false)
  const [result, setResult] = useState<PatchResult | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [logs])

  useEffect(() => {
    if (!isTauri()) {
      return
    }
    let unlisten: (() => void) | undefined
    listen<string>("wwr-log", (event) => {
      setLogs((prev) => [...prev, event.payload])
    }).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [])

  async function onRemove() {
    setRunning(true)
    setResult(null)
    setLogs(["Starting watermark removal…"])

    if (!isTauri()) {
      setLogs((prev) => [
        ...prev,
        "Browser preview only — open the built Windows app to patch Explorer.",
        "In the desktop build, logs stream from the Rust patcher.",
      ])
      setResult({
        ok: false,
        message: "Not running inside the Windows desktop app.",
        rva: null,
      })
      setRunning(false)
      return
    }

    try {
      const next = await invoke<PatchResult>("remove_watermark")
      setResult(next)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setLogs((prev) => [...prev, `ERROR: ${message}`])
      setResult({ ok: false, message, rva: null })
    } finally {
      setRunning(false)
    }
  }

  return (
    <div className="flex min-h-svh flex-col bg-background p-6">
      <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col gap-4">
        <div className="flex items-start gap-3">
          <img
            src="/app-icon.png"
            alt=""
            className="size-12 rounded-xl ring-1 ring-foreground/10"
          />
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-lg font-medium tracking-tight">
                Windows Watermark Remover
              </h1>
              <Badge variant="secondary">Insider</Badge>
            </div>
            <p className="text-sm text-muted-foreground">
              Patches{" "}
              <span className="font-mono text-xs">
                CDesktopWatermark::s_DesktopBuildPaint
              </span>{" "}
              in Explorer and shows every step in the log.
            </p>
          </div>
        </div>

        <Card className="flex min-h-0 flex-1 flex-col">
          <CardHeader className="border-b">
            <CardTitle>Remove evaluation watermark</CardTitle>
            <CardDescription>
              Hides the corner “Evaluation copy” text on Windows Insider
              builds. Does not activate Windows.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex min-h-0 flex-1 flex-col gap-3">
            <div className="flex items-center justify-between gap-2">
              <p className="text-sm font-medium">Activity log</p>
              <Badge variant="outline">{logs.length} lines</Badge>
            </div>
            <ScrollArea className="h-72 rounded-lg border bg-muted/30">
              <div className="flex flex-col gap-1 p-3 font-mono text-xs leading-relaxed">
                {logs.map((line, index) => (
                  <div
                    key={`${index}-${line.slice(0, 24)}`}
                    className="text-foreground/90"
                  >
                    <span className="text-muted-foreground">
                      {String(index + 1).padStart(3, "0")}
                    </span>{" "}
                    {line}
                  </div>
                ))}
                <div ref={bottomRef} />
              </div>
            </ScrollArea>

            {result ? (
              <Alert variant={result.ok ? "default" : "destructive"}>
                {result.ok ? <CheckCircle2Icon /> : <AlertCircleIcon />}
                <AlertTitle>{result.ok ? "Patched" : "Failed"}</AlertTitle>
                <AlertDescription>
                  {result.message}
                  {result.rva ? ` RVA ${result.rva}` : null}
                </AlertDescription>
              </Alert>
            ) : null}
          </CardContent>
          <Separator />
          <CardFooter className="justify-between gap-3">
            <p className="text-xs text-muted-foreground">
              Memory patch only. Re-applies at logon via HKCU Run.
            </p>
            <Button onClick={onRemove} disabled={running}>
              {running ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <EraserIcon data-icon="inline-start" />
              )}
              {running ? "Working…" : "Remove watermark"}
            </Button>
          </CardFooter>
        </Card>
      </div>
    </div>
  )
}

export default App
