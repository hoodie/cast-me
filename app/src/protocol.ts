export interface BaseCommand {
    type: string;
    payload: string;
}

type TypeOfCmd<T extends { type: unknown }> = T['type'];

export interface OfferCommand {
    type: "offer";
    payload: RTCSessionDescriptionInit;
}

export interface AnswerCommand {
    type: "answer";
    payload: RTCSessionDescriptionInit;
}

export interface CandidateCommand {
    type: "candidate";
    payload: RTCIceCandidate | null;
}

export type Command = OfferCommand | AnswerCommand | CandidateCommand;
export type CommandTypes = TypeOfCmd<Command>;

const isCommand = (cmd: unknown): cmd is { type: string } =>
    typeof cmd === 'object' && 'type' in cmd

const isXCommand = <C>(tag: string) => (cmd: unknown): cmd is C =>
    isCommand(cmd) && cmd.type === tag

export const isOfferCommand = isXCommand<OfferCommand>('offer')
export const isAnswerCommand = isXCommand<AnswerCommand>('answer')
export const isCandidateCommand = isXCommand<CandidateCommand>('candidate')


/// peer messages : things the server says to peers

const isXMessage = <C>(tag: string) => (cmd: unknown): cmd is C =>
    typeof cmd === 'object' && tag in cmd
    // isCommand(cmd) && tag in cmd

export interface WelcomeMsg {
    welcome: string
}

export interface ConnectedMsg {
    connected: string
}

export interface ByeMsg {
    bye: {
        reason: string;
    }
}


export const isWelcomeMsg = isXMessage<WelcomeMsg>('welcome');
export const isConnectedMsg = isXMessage<ConnectedMsg>('connected');
export const isByeMsg = isXMessage<ByeMsg>('bye');