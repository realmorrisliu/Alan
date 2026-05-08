import Darwin

#if os(macOS)
enum AlanMacAppStartup {
    static func acquireSingletonOrTerminate() -> AlanAppSingletonGuard {
        do {
            switch try AlanAppSingletonGuard.acquire() {
            case .acquired(let guardHandle):
                return guardHandle
            case .alreadyRunning:
                AlanAppSingletonGuard.activateExistingInstance()
                Darwin.exit(0)
            }
        } catch {
            fatalError("Alan could not acquire the macOS app singleton lock: \(error)")
        }
    }
}
#endif
