export interface BaseCommand {
    type: string;
    payload: string;
}

type TypeOfCmd<T extends { type: unknown }> = T['type'];
export type ThingOfType<C, T extends string> = C extends { type: T } ? C : never;
export type CommandOfType<T extends CommandTypes> = ThingOfType<Command, T>
export type PayloadOfType<T extends CommandTypes> = CommandOfType<T>['payload']

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

export interface GoFullscreenCommand {
    type: "go-full-screen";
    payload: boolean
}

export type Command = OfferCommand | AnswerCommand | CandidateCommand | GoFullscreenCommand;
export type CommandTypes = TypeOfCmd<Command>;

const isCommand = (cmd: unknown): cmd is { type: string } =>
    typeof cmd === 'object' && 'type' in cmd

export const isXCommand = <C>(tag: CommandTypes) => (cmd: unknown): cmd is C =>
    isCommand(cmd) && cmd.type === tag

/// peer messages : things the server says to peers

const isXMessage = <C>(tag: string) => (cmd: unknown): cmd is C =>
    typeof cmd === 'object' && tag in cmd

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