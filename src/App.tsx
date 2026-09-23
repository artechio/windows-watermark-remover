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
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"

type PatchResult = {
  ok: boolean
  message: string
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

export function App() {
  const [logs, setLogs] = useState<string[]>([
    "Ready. Click Remove watermark when you want to hide the evaluation text.",
  ])
  const [running, setRunning] = useState(false)
  const [result, setResult] = useState<PatchResult | null>(null)
  const [startupEnabled, setStartupEnabled] = useState(false)
  const [startupBusy, setStartupBusy] = useState(false)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [logs])

  useEffect(() => {
    if (!isTauri()) {
      return
    }

    let unlistenLog: (() => void) | undefined
    let unlistenAuto: (() => void) | undefined

    listen<string>("wwr-log", (event) => {
      setLogs((prev) => [...prev, event.payload])
    }).then((fn) => {
      unlistenLog = fn
    })

    listen<PatchResult>("wwr-auto-apply", (event) => {
      setResult(event.payload)
      setRunning(false)
    }).then((fn) => {
      unlistenAuto = fn
    })

    invoke<boolean>("get_startup_enabled")
      .then(setStartupEnabled)
      .catch(() => setStartupEnabled(false))

    invoke<boolean>("should_auto_apply")
      .then((auto) => {
        if (auto) {
          setRunning(true)
          setLogs(["Signing in… removing the watermark automatically."])
        }
      })
      .catch(() => undefined)

    return () => {
      unlistenLog?.()
      unlistenAuto?.()
    }
  }, [])

  async function onRemove() {
    setRunning(true)
    setResult(null)
    setLogs(["Starting watermark removal…"])

    if (!isTauri()) {
      setLogs((prev) => [
        ...prev,
        "This preview cannot change Windows. Open the desktop app to remove the watermark.",
      ])
      setResult({
        ok: false,
        message: "Not running inside the Windows desktop app.",
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
      setResult({ ok: false, message })
    } finally {
      setRunning(false)
    }
  }

  async function onStartupChange(next: boolean) {
    if (!isTauri()) {
      setStartupEnabled(next)
      return
    }
    setStartupBusy(true)
    try {
      const enabled = await invoke<boolean>("set_startup_enabled", {
        enabled: next,
      })
      setStartupEnabled(enabled)
      setLogs((prev) => [
        ...prev,
        enabled
          ? "Sign-in reapply on. A silent copy was saved under your user folder and will run after you sign in."
          : "Sign-in reapply off. The silent copy and startup entry were removed.",
      ])
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setLogs((prev) => [...prev, `ERROR: ${message}`])
    } finally {
      setStartupBusy(false)
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
              Hide the evaluation copy text in the corner of your Insider
              desktop. Clear steps show up in the log as you go.
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
            <ScrollArea className="h-64 rounded-lg border bg-muted/30">
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
                <AlertTitle>{result.ok ? "Done" : "Something went wrong"}</AlertTitle>
                <AlertDescription>{result.message}</AlertDescription>
              </Alert>
            ) : null}

            {result?.ok || startupEnabled ? (
              <div className="flex items-start justify-between gap-3 rounded-lg border p-3">
                <div className="flex min-w-0 flex-col gap-1">
                  <Label htmlFor="startup-toggle">
                    Remove again after I sign in
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Optional. Saves a silent copy in your user folder and runs
                    it after sign-in — no clicks, and it still works if you
                    delete this download.
                  </p>
                </div>
                <Switch
                  id="startup-toggle"
                  checked={startupEnabled}
                  disabled={startupBusy}
                  onCheckedChange={onStartupChange}
                />
              </div>
            ) : null}
          </CardContent>
          <Separator />
          <CardFooter className="justify-between gap-3">
            <p className="text-xs text-muted-foreground">
              No installer. Does not change Windows activation.
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
