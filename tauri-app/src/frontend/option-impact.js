// Resource guidance shares the same reviewed, version-aware source as placement.
import {guideFor} from './option-guide.js';
export function impactFor(option){const {speed,vram,ram}=guideFor(option);return {speed,vram,ram};}
