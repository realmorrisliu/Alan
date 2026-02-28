import Foundation

struct HealthResponse: Decodable {
    let status: String
}

struct SessionResponse: Decodable {
    let sessionID: String

    private enum CodingKeys: String, CodingKey {
        case sessionID = "session_id"
        case id
        case session
    }

    private enum SessionContainerKeys: String, CodingKey {
        case id
        case sessionID = "session_id"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if let value = try container.decodeIfPresent(String.self, forKey: .sessionID) {
            sessionID = value
            return
        }
        if let value = try container.decodeIfPresent(String.self, forKey: .id) {
            sessionID = value
            return
        }
        if container.contains(.session) {
            let nested = try container.nestedContainer(keyedBy: SessionContainerKeys.self, forKey: .session)
            if let value = try nested.decodeIfPresent(String.self, forKey: .sessionID) {
                sessionID = value
                return
            }
            if let value = try nested.decodeIfPresent(String.self, forKey: .id) {
                sessionID = value
                return
            }
        }
        throw AlanAPIError.missingSessionID
    }
}

struct SubmitResponse: Decodable {
    let submissionID: String
    let accepted: Bool

    private enum CodingKeys: String, CodingKey {
        case submissionID = "submission_id"
        case accepted
    }
}

struct ReadEventsResponse: Decodable {
    let sessionID: String
    let gap: Bool
    let oldestEventID: String?
    let latestEventID: String?
    let events: [SessionEventEnvelope]

    private enum CodingKeys: String, CodingKey {
        case sessionID = "session_id"
        case gap
        case oldestEventID = "oldest_event_id"
        case latestEventID = "latest_event_id"
        case events
    }
}

struct SessionEventEnvelope: Decodable {
    let eventID: String
    let type: String
    let chunk: String?
    let content: String?
    let message: String?
    let isFinal: Bool?
    let recoverable: Bool?
    let replayFromEventID: String?

    var textChunk: String? {
        chunk ?? content
    }

    private enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
        case chunk
        case content
        case message
        case isFinal = "is_final"
        case recoverable
        case replayFromEventID = "replay_from_event_id"
    }
}

enum AlanAPIError: LocalizedError {
    case invalidURL(String)
    case invalidResponse
    case unexpectedStatusCode(Int, String)
    case missingSessionID
    case invalidTextInput
    case responseTimeout

    var errorDescription: String? {
        switch self {
        case .invalidURL(let value):
            return "Invalid server URL: \(value)"
        case .invalidResponse:
            return "Invalid server response"
        case .unexpectedStatusCode(let code, let body):
            return "Server returned \(code): \(body)"
        case .missingSessionID:
            return "Session response did not include an id"
        case .invalidTextInput:
            return "Message cannot be empty"
        case .responseTimeout:
            return "Timed out waiting for assistant response"
        }
    }
}

struct AlanAPIClient {
    private let baseURL: URL
    private let session: URLSession
    private let decoder = JSONDecoder()

    init(baseURLString: String, session: URLSession = .shared) throws {
        let trimmed = baseURLString.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let url = URL(string: trimmed) else {
            throw AlanAPIError.invalidURL(baseURLString)
        }

        self.baseURL = url
        self.session = session
    }

    func checkHealth() async throws -> HealthResponse {
        let requestURL = endpointURL(pathComponents: ["health"])
        let (data, response) = try await session.data(from: requestURL)
        try validate(response: response, data: data)

        if let decoded = try? decoder.decode(HealthResponse.self, from: data) {
            return decoded
        }

        let body = String(data: data, encoding: .utf8) ?? "ok"
        return HealthResponse(status: body)
    }

    func createSession() async throws -> SessionResponse {
        let requestURL = endpointURL(pathComponents: ["api", "v1", "sessions"])
        var request = URLRequest(url: requestURL)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = #"{"governance":{"profile":"conservative"}}"#.data(using: .utf8)

        let (data, response) = try await session.data(for: request)
        try validate(response: response, data: data)
        return try decoder.decode(SessionResponse.self, from: data)
    }

    func submitInput(sessionID: String, text: String) async throws -> SubmitResponse {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw AlanAPIError.invalidTextInput
        }

        let requestURL = endpointURL(pathComponents: ["api", "v1", "sessions", sessionID, "submit"])
        var request = URLRequest(url: requestURL)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(
            withJSONObject: [
                "op": [
                    "type": "turn",
                    "parts": [["type": "text", "text": trimmed]],
                ],
            ]
        )

        let (data, response) = try await session.data(for: request)
        try validate(response: response, data: data)
        return try decoder.decode(SubmitResponse.self, from: data)
    }

    func readEvents(sessionID: String, afterEventID: String?) async throws -> ReadEventsResponse {
        var components = URLComponents(
            url: endpointURL(pathComponents: ["api", "v1", "sessions", sessionID, "events", "read"]),
            resolvingAgainstBaseURL: false
        )
        var queryItems = [URLQueryItem(name: "limit", value: "200")]
        if let afterEventID, !afterEventID.isEmpty {
            queryItems.append(URLQueryItem(name: "after_event_id", value: afterEventID))
        }
        components?.queryItems = queryItems
        guard let requestURL = components?.url else {
            throw AlanAPIError.invalidURL(baseURL.absoluteString)
        }

        let (data, response) = try await session.data(from: requestURL)
        try validate(response: response, data: data)
        return try decoder.decode(ReadEventsResponse.self, from: data)
    }

    private func endpointURL(pathComponents: [String]) -> URL {
        pathComponents.reduce(baseURL) { partial, component in
            partial.appendingPathComponent(component)
        }
    }

    private func validate(response: URLResponse, data: Data) throws {
        guard let http = response as? HTTPURLResponse else {
            throw AlanAPIError.invalidResponse
        }

        guard (200..<300).contains(http.statusCode) else {
            let body = String(data: data, encoding: .utf8) ?? "<no body>"
            throw AlanAPIError.unexpectedStatusCode(http.statusCode, body)
        }
    }
}
