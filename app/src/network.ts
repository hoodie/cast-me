import { webSocket } from "rxjs/webSocket";
import { pluck, filter, map, first } from "rxjs/operators";
import {
    AnswerCommand,
    ByeMsg,
    CandidateCommand,
    Command,
    CommandOfType,
    CommandTypes,
    GoFullscreenCommand,
    isByeMsg,
    isConnectedMsg,
    isWelcomeMsg,
    isXCommand,
    OfferCommand,
    PayloadOfType,
} from "./protocol";
import type { Observable } from "rxjs";

const socket = webSocket<Command>(`wss://${location.host}/ws`);
socket.next("subscribed" as any); // inital message to server, actually ignored

type Fn<P, R> = (x: P) => R;
const not =
    <T>(f: Fn<T, boolean>) =>
        (x: T) =>
            !f(x);

const isProtocolCmd = (command: any): command is Command =>
    typeof command === "object" &&
    ("welcome" in command ||
        "connect" in command ||
        "connected" in command ||
        "bye" in command);

// received bye
export const byeReceived: Observable<ByeMsg['bye']> = socket.pipe(
    filter(isByeMsg),
    pluck('bye'),
    first()
);

// your peer's ID
export const connectReceived: Observable<string> = socket.pipe(
    filter(isConnectedMsg),
    pluck("connected"),
    first()
);

// your ID
export const ownPeerId: Observable<string> = socket.pipe(
    filter(isWelcomeMsg),
    map(({ welcome }) => welcome)
);

export const sendAsRaw = (payload) => socket.next(payload);

export const payloadMsg = socket.pipe(filter(not(isProtocolCmd)));

export const isOfferCommand = isXCommand<OfferCommand>('offer');
export const isAnswerCommand = isXCommand<AnswerCommand>('answer');
export const isCandidateCommand = isXCommand<CandidateCommand>('candidate');
export const isGoFullscreenCommand = isXCommand<GoFullscreenCommand>('go-full-screen');

// your should turn on fullscreen
export const goFullScreenReceived: Observable<boolean> = socket.pipe(
    filter(isGoFullscreenCommand),
    pluck('payload')
);


export const offers = socket.pipe(filter(isOfferCommand), pluck('payload'));
export const answers = socket.pipe(filter(isAnswerCommand), pluck('payload'));
export const candidates = socket.pipe(
    filter(isCandidateCommand),
    pluck("payload")
);
export const fullscreen = socket.pipe(filter(isGoFullscreenCommand), pluck('payload'));

export const sendAsType = <T extends CommandTypes>(type: T) => (payload: PayloadOfType<T>) =>
    socket.next({ type, payload } as CommandOfType<T>);

export const sendAsOffer = sendAsType("offer");
export const sendAsAnswer = sendAsType("answer");
export const sendAsCandidate = sendAsType("candidate");
export const sendGoFullscreenCommand = sendAsType('go-full-screen');

window["API"] = {
    sendAsOffer,
    sendAsAnswer,
    sendAsCandidate,
    offers,
    answers,
    candidates,
    sendAsRaw,
};
