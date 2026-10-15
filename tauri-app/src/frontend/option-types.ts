import type {OptionGuide} from './option-guide-types';
export interface ServerOption {flag:string;aliases:string[];syntax:string;section:string;description:string;choices:string[];arity:number;defaultValue:string;category:string;searchText:string;guide:OptionGuide}
