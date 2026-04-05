import {
  ChevronDown,
  Gauge,
  Pause,
  Play,
  StepBack,
  StepForward,
  Workflow,
} from "lucide-react"
import { PLAYBACK_SPEEDS } from "../utils/constants.js"
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
  ScrollArea,
  Separator,
  Slider,
} from "./ui"

function StepDropdown({ currentStep, onStepChange, stepOptions, totalEvents }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          className="playback-bar__picker gap-1.5 text-xs"
          size="sm"
          type="button"
          variant="outline"
        >
          Step {currentStep} / {totalEvents}
          <ChevronDown className="h-3 w-3 opacity-70" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        className="w-[320px] border-border bg-popover/98 p-1"
      >
        <ScrollArea className="max-h-72">
          <DropdownMenuRadioGroup
            onValueChange={(value) => onStepChange(Number(value))}
            value={String(Math.max(0, currentStep - 1))}
          >
            {stepOptions.map((step) => (
              <DropdownMenuRadioItem
                className="py-2 text-sm"
                key={step.value}
                value={String(step.value)}
              >
                {step.label}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </ScrollArea>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function PipelineDropdown({ onRunSelect, runs, selectedRun }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          className="playback-bar__picker gap-1.5 text-xs"
          size="sm"
          type="button"
          variant="outline"
        >
          <Workflow className="h-3 w-3" />
          {selectedRun?.label ?? "Pipeline"}
          <ChevronDown className="h-3 w-3 opacity-70" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        className="w-[260px] border-border bg-popover/98 p-1"
      >
        <ScrollArea className="max-h-72">
          <DropdownMenuRadioGroup
            onValueChange={onRunSelect}
            value={selectedRun?.id ?? ""}
          >
            {runs.map((run) => (
              <DropdownMenuRadioItem className="py-2" key={run.id} value={run.id}>
                <div className="flex w-full items-center justify-between gap-3">
                  <span className="truncate">{run.label}</span>
                  <span className="text-xs text-muted-foreground">
                    {run.events.length}
                  </span>
                </div>
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </ScrollArea>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function SpeedDropdown({ onSpeedChange, playbackSpeed }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          className="playback-bar__picker gap-1.5 text-xs"
          size="sm"
          type="button"
          variant="outline"
        >
          <Gauge className="h-3 w-3" />
          {playbackSpeed}x
          <ChevronDown className="h-3 w-3 opacity-70" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="end"
        className="w-[120px] border-border bg-popover/98 p-1"
      >
        <ScrollArea className="max-h-72">
          <DropdownMenuRadioGroup
            onValueChange={(value) => onSpeedChange(Number(value))}
            value={String(playbackSpeed)}
          >
            {PLAYBACK_SPEEDS.map((speed) => (
              <DropdownMenuRadioItem
                className="py-2"
                key={speed}
                value={String(speed)}
              >
                {speed}x
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </ScrollArea>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export default function PlaybackControls({
  currentStepLabel,
  hasDetailsPanel,
  isPlaying,
  onPause,
  onPlay,
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
        </div>

        <Slider
          className="playback-bar__slider"
          max={totalEvents}
          min={1}
          onValueChange={([value]) => onStepChange(value - 1)}
          step={1}
          value={[currentStep]}
        />

        {currentStepLabel ? (
          <div className="playback-bar__label">{currentStepLabel}</div>
        ) : null}
      </div>
    </footer>
  )
}
