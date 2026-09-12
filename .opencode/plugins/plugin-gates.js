import { existsSync, readFileSync } from 'fs';
import { resolve } from 'path';
import {
  checkBriefQuality,
  checkCodeElegance,
  checkOutcomeQuality,
  checkPersonaQuality,
  checkSecurityBaseline,
} from './plugin-quality-checks.js';
import {
  isBriefFile,
  isOutcomeFile,
  isPersonaFile,
  isSourceCodePath,
  isSrcFile,
} from './plugin-paths.js';
import { markerExists, markerValid, touchMarker } from './plugin-markers.js';

const HALF_HOUR = 1800000;
const TWO_HOURS = 7200000;
const ONE_DAY = 86400000;
const ONE_HOUR = 3600000;

function readMarker(name) {
  return resolve(process.cwd(), '.odd', name);
}

function isVerifiedUpgrade(event, filePath) {
  if (!filePath.includes('state.json')) return false;

  if (event.tool?.name === 'Write') {
    const next = event.input?.content || '';
    const nextCount = (next.match(/"buildStatus"\s*:\s*"verified"/g) || []).length;
    const prevCount = existsSync(filePath)
      ? (readFileSync(filePath, 'utf8').match(/"buildStatus"\s*:\s*"verified"/g) || []).length
      : 0;
    return nextCount > prevCount;
  }

  const previous = event.input?.old_string || '';
  const next = event.input?.new_string || '';
  return (next.match(/"verified"/g) || []).length > (previous.match(/"verified"/g) || []).length;
}

function blocksBuildPhaseTransition(newContent) {
  if (!/"currentPhase"\s*:\s*"build"/.test(newContent)) return false;
  return (
    !/"techStackDecided"\s*:\s*true/.test(newContent) ||
    !/"designApproachDecided"\s*:\s*true/.test(newContent) ||
    !/"architectureDocGenerated"\s*:\s*true/.test(newContent)
  );
}

export function createBeforeHook(readState) {
  return (event) => {
    const state = readState();
    if (!state) return;

    const toolName = event.tool?.name || '';
    const filePath = event.input?.file_path || event.input?.path || '';

    if (toolName === 'Agent' && state.currentPhase === 'build') {
      const prompt = (event.input?.prompt || '').slice(0, 300).toLowerCase();
      const isSupportDispatch = prompt.match(/session.brief|session-brief|generate.*brief|write.*brief|odd-sync|fix.*hook|update.*hook/);

      if (!state.briefConfirmed && !prompt.match(/session.brief|session-brief|generate.*brief|write.*brief/)) {
        return { blocked: true, message: 'ODD BRIEF GATE: Build agents blocked — session brief not confirmed. Generate and confirm the brief first.' };
      }

      if (state.briefConfirmed && !markerValid(readMarker('.odd-flow-phase-synced'), HALF_HOUR)) {
        return { blocked: true, message: 'TRELLIS BUILD GATE: Brief confirmed but odd-flow not synced. Run /trellis-sync first.' };
      }

      if (markerValid(readMarker('.odd-quick-fix'), ONE_HOUR)) return;

      if (
        state.briefConfirmed &&
        !markerValid(readMarker('.odd-flow-agents-ready'), TWO_HOURS) &&
        !isSupportDispatch
      ) {
        return { blocked: true, message: 'ODD BUILD GATE: Executor agents blocked — odd-flow swarm not initialised. Run the full swarm init sequence before dispatching Agent tool work.' };
      }

      if (state.briefConfirmed && !isSupportDispatch && markerValid(readMarker('.odd-flow-agents-ready'), TWO_HOURS)) {
        touchMarker('.odd-flow-agent-token');
        touchMarker('.odd-flow-last-agent-dispatch');
      }
    }

    if ((toolName === 'Write' || toolName === 'Edit') && state.currentPhase === 'build' && isSourceCodePath(filePath)) {
      if (!markerValid(readMarker('.odd-flow-swarm-active'), ONE_DAY)) {
        return { blocked: true, message: `ODD SWARM WRITE GATE: Source writes blocked — no active build session. Run *build first. File: ${filePath}` };
      }

      if (state.debugSession || markerValid(readMarker('.odd-quick-fix'), ONE_HOUR)) return;

      if (!markerValid(readMarker('.odd-flow-agent-token'), TWO_HOURS)) {
        return { blocked: true, message: `ODD SWARM WRITE GATE: Source writes blocked — no agent write token. Dispatch build work with the Agent tool after swarm init. odd-flow agent_spawn only sets coordination metadata. File: ${filePath}` };
      }
    }

    if ((toolName === 'Edit' || toolName === 'Write') && filePath.includes('state.json')) {
      const newContent = event.input?.new_string || event.input?.content || '';

      if (blocksBuildPhaseTransition(newContent)) {
        return { blocked: true, message: 'ODD PLAN COMPLETE GATE: Finish technical architecture, design approach, and architecture docs before entering build phase.' };
      }

      if (isVerifiedUpgrade(event, filePath)) {
        if (state.buildMode === 'debug') {
          return { blocked: true, message: 'ODD VERIFY GATE: Cannot mark outcomes as verified while debug mode is active. Return to verification first.' };
        }

        if (!state.verificationConfirmed) {
          return { blocked: true, message: 'ODD VERIFY GATE: Cannot mark outcomes as verified. Walk through the verification checklist first.' };
        }
      }

      if (/"briefConfirmed"\s*:\s*true/.test(newContent) && !markerExists('.odd-flow-brief-stored')) {
        return { blocked: true, message: 'ODD CONFIRM GATE: Brief not stored in odd-flow memory. Store it first.' };
      }
    }
  };
}

export function createAfterHook(readState, readLastCommit, writeState) {
  return (event) => {
    const toolName = event.tool?.name || '';
    const filePath = event.input?.file_path || event.input?.path || '';
    const warnings = [];

    if (toolName === 'Write' && isOutcomeFile(filePath)) warnings.push(...checkOutcomeQuality(filePath));
    if (toolName === 'Write' && isPersonaFile(filePath)) warnings.push(...checkPersonaQuality(filePath));
    if ((toolName === 'Write' || toolName === 'Edit') && isSrcFile(filePath)) {
      if (state?.currentPhase === 'build') touchMarker('.odd-flow-agent-token');
      warnings.push(...checkCodeElegance(filePath));
      warnings.push(...checkSecurityBaseline(filePath));
    }
    if (toolName === 'Write' && isBriefFile(filePath)) warnings.push(...checkBriefQuality(filePath));

    const state = readState();

    if (toolName === 'Bash') {
      const cmd = event.input?.command || '';
      if (/git\s+commit/.test(cmd) && event.exitCode === 0 && state) {
        state.lastCommit = readLastCommit();
        state.lastSaved = new Date().toISOString();
        state.lastCommitAt = state.lastSaved;
        writeState(state);
        touchMarker('.odd-flow-phase-synced');
      }
    }

    if (toolName === 'mcp__odd-flow__memory_store' && state?.currentPhase === 'build') {
      if (event.input?.key?.startsWith('trellis-session-brief-')) touchMarker('.odd-flow-brief-stored');
    }

    if (toolName === 'mcp__odd-flow__coordination_sync' && state?.currentPhase === 'build') {
      touchMarker('.odd-flow-swarm-active');
      touchMarker('.odd-flow-agents-ready');
      touchMarker('.odd-flow-phase-synced');
      touchMarker('.odd-flow-agent-token');
    }

    if (warnings.length > 0) return { warnings };
  };
}

export function createChatHook(readState) {
  return () => {
    const state = readState();
    if (!state || state.currentPhase !== 'build') return;

    if (state.buildMode === 'debug') {
      return {
        warnings: [
          `ODD DEBUG MODE: ${state.debugStrategy || 'Choose a strategy'} — keep the fix inside the active outcome and return to verification when ready.`,
        ],
      };
    }

    const swarmPath = readMarker('.odd-flow-swarm-active');
    if (!markerValid(swarmPath, ONE_DAY)) {
      const stale = markerExists('.odd-flow-swarm-active') ? ' (marker expired — re-init required)' : '';
      return { warnings: [`ODD SWARM GUARD: You are in build phase. The odd-flow swarm must be initialised before build work.${stale}`] };
    }
  };
}
