import { useEffect, useRef, useState } from "react"
import { Gauge, Pause, Play, StepBack, StepForward, Workflow } from "lucide-react"
import { PLAYBACK_SPEEDS } from "../utils/constants.js"
import {
  Button,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Separator,
  Slider,
} from "./ui"

function StepDropdown({ currentStep, onStepChange, stepOptions, totalEvents }) {
  return (
    <Select
      onValueChange={(value) => onStepChange(Number(value))}
      value={String(Math.max(0, currentStep - 1))}
    >
      <SelectTrigger className="playback-bar__picker h-8 gap-1.5 text-xs">
        <SelectValue>
          Step {currentStep} / {totalEvents}
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="max-h-72 w-[320px] border-border bg-popover/98">
        {stepOptions.map((step) => (
          <SelectItem className="py-2 text-sm" key={step.value} value={String(step.value)}>
            {step.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

function PipelineDropdown({ onRunSelect, runs, selectedRun }) {
  return (
    <Select onValueChange={onRunSelect} value={selectedRun?.id ?? ""}>
      <SelectTrigger className="playback-bar__picker h-8 gap-1.5 text-xs">
        <Workflow className="h-3 w-3 shrink-0" />
        <SelectValue placeholder="Pipeline">
          {selectedRun?.label ?? "Pipeline"}
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="max-h-72 w-[260px] border-border bg-popover/98">
        {runs.map((run) => (
          <SelectItem className="py-2" key={run.id} value={run.id}>
            <div className="flex w-full items-center justify-between gap-3">
              <span className="truncate">{run.label}</span>
              <span className="text-xs text-muted-foreground">{run.events.length}</span>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

function SpeedDropdown({ onSpeedChange, playbackSpeed }) {
  return (
    <Select
      onValueChange={(value) => onSpeedChange(Number(value))}
      value={String(playbackSpeed)}
    >
      <SelectTrigger className="playback-bar__picker h-8 w-[100px] gap-1.5 text-xs">
        <Gauge className="h-3 w-3 shrink-0" />
        <SelectValue>{playbackSpeed}x</SelectValue>
      </SelectTrigger>
      <SelectContent className="max-h-72 w-[100px] border-border bg-popover/98">
        {PLAYBACK_SPEEDS.map((speed) => (
          <SelectItem className="py-2" key={speed} value={String(speed)}>
            {speed}x
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

export default function PlaybackControls({
  canvasMode,
  currentStepLabel,
  hasDetailsPanel,
  isPlaying,
  onPause,
  onPlay,
  onCanvasModeChange,
  onRunSelect,
  onSkipEnd,
  onSkipStart,
  onSpeedChange,
  onStepChange,
  playbackIndex,
  playbackSpeed,
  runs,
  selectedRun,
  stepOptions,
  totalEvents,
}) {
  const currentStep = Math.max(1, playbackIndex + 1)
  const [displayStep, setDisplayStep] = useState(currentStep)
  const displayStepRef = useRef(currentStep)

  useEffect(() => {
    displayStepRef.current = displayStep
  }, [displayStep])

  useEffect(() => {
    const startValue = displayStepRef.current
    const targetValue = currentStep

    if (Math.abs(targetValue - startValue) < 0.001) {
      setDisplayStep(targetValue)
      return
    }

    const durationMs = Math.min(420, 180 + Math.abs(targetValue - startValue) * 70)
    let animationFrameId = 0
    let startTime = 0

    const animate = (timestamp) => {
      if (!startTime) {
        startTime = timestamp
      }

      const progress = Math.min(1, (timestamp - startTime) / durationMs)
      const eased = 1 - Math.pow(1 - progress, 3)
      const nextValue = startValue + (targetValue - startValue) * eased

      displayStepRef.current = nextValue
      setDisplayStep(nextValue)

      if (progress < 1) {
        animationFrameId = window.requestAnimationFrame(animate)
      }
    }

    animationFrameId = window.requestAnimationFrame(animate)
    return () => {
      window.cancelAnimationFrame(animationFrameId)
    }
  }, [currentStep])

  const togglePlayback = () => {
    if (isPlaying) {
      onPause()
      return
    }

    onPlay()
  }

  return (
    <footer
      className={`playback-bar ${hasDetailsPanel ? "playback-bar--with-panel" : ""}`}
    >
      <div className="playback-bar__controls">
        <Button
          onClick={onSkipStart}
          size="icon"
          type="button"
          variant="outline"
        >
          <StepBack className="h-3.5 w-3.5" />
        </Button>
        <Button
          className="playback-bar__play"
          onClick={togglePlayback}
          size="icon"
          type="button"
        >
          {isPlaying ? (
            <Pause className="h-3.5 w-3.5" />
          ) : (
            <Play className="h-3.5 w-3.5" />
          )}
        </Button>
        <Button onClick={onSkipEnd} size="icon" type="button" variant="outline">
          <StepForward className="h-3.5 w-3.5" />
        </Button>
      </div>

      <Separator className="playback-bar__separator" orientation="vertical" />

      <div className="playback-bar__timeline">
        <div className="playback-bar__toolbar">
          <div className="playback-bar__toolbar-group">
            <PipelineDropdown
              onRunSelect={onRunSelect}
              runs={runs}
              selectedRun={selectedRun}
            />
            <StepDropdown
              currentStep={currentStep}
              onStepChange={onStepChange}
              stepOptions={stepOptions}
              totalEvents={totalEvents}
            />
          </div>

          <SpeedDropdown
            onSpeedChange={onSpeedChange}
            playbackSpeed={playbackSpeed}
          />

          <div
            aria-label="Canvas interaction mode"
            className="playback-bar__mode-toggle"
            role="group"
          >
            <Button
              className={`playback-bar__mode-button ${canvasMode === "pan-canvas" ? "is-active" : ""}`}
              onClick={() => onCanvasModeChange("pan-canvas")}
              size="sm"
              type="button"
              variant="outline"
            >
              Pan
            </Button>
            <Button
              className={`playback-bar__mode-button ${canvasMode === "move-nodes" ? "is-active" : ""}`}
              onClick={() => onCanvasModeChange("move-nodes")}
              size="sm"
              type="button"
              variant="outline"
            >
              Nodes
            </Button>
          </div>
        </div>

        <Slider
          className="playback-bar__slider"
          max={totalEvents}
          min={1}
          onValueChange={([value]) => onStepChange(value - 1)}
          step={1}
          value={[displayStep]}
        />

        {currentStepLabel ? (
          <div className="playback-bar__label">{currentStepLabel}</div>
        ) : null}
      </div>
    </footer>
  )
}
