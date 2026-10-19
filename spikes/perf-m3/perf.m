// Loads a URL in a visible WKWebView, awaits window.measure() and prints its result.
// The window must be on screen for requestAnimationFrame to run.
#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>

static NSString *const kErrorCollector =
    @"window.__errors = [];"
     "addEventListener('error', (e) => window.__errors.push(String(e.message || e)));"
     "addEventListener('unhandledrejection', (e) => window.__errors.push(String(e.reason)));";

@interface Delegate : NSObject <WKNavigationDelegate>
@end
@implementation Delegate
// Module scripts may finish after the navigation does, so poll for the page's entry point.
- (void)tryMeasure:(WKWebView *)webView attempt:(int)attempt {
    [webView evaluateJavaScript:@"typeof window.measure === 'function'" completionHandler:^(id ready, NSError *error) {
        if ([ready boolValue]) {
            [webView callAsyncJavaScript:@"return await window.measure();"
                               arguments:nil
                                 inFrame:nil
                          inContentWorld:WKContentWorld.pageWorld
                       completionHandler:^(id result, NSError *inner) {
                if (inner) { printf("error: %s\n", inner.description.UTF8String); }
                else { printf("%s\n", [[NSString stringWithFormat:@"%@", result] UTF8String]); }
                fflush(stdout);
                exit(0);
            }];
        } else if (attempt < 25) {
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 200 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
                [self tryMeasure:webView attempt:attempt + 1];
            });
        } else {
            [webView evaluateJavaScript:@"JSON.stringify({ errors: window.__errors, scripts: [...document.scripts].map((s) => s.src) })"
                      completionHandler:^(id report, NSError *inner) {
                printf("no measure(): %s\n", [[NSString stringWithFormat:@"%@", report ?: inner] UTF8String]);
                exit(1);
            }];
        }
    }];
}
- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation {
    [self tryMeasure:webView attempt:0];
}
@end

int main(int argc, char **argv) {
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        [app setActivationPolicy:NSApplicationActivationPolicyAccessory];
        NSString *path = [NSString stringWithUTF8String:argv[1]];
        NSRect screen = NSScreen.mainScreen.visibleFrame;
        NSRect frame = NSMakeRect(NSMaxX(screen) - 920, NSMinY(screen), 900, 700);
        NSWindow *window = [[NSWindow alloc] initWithContentRect:frame styleMask:NSWindowStyleMaskTitled backing:NSBackingStoreBuffered defer:NO];
        window.title = @"Aside perf harness";
        WKWebViewConfiguration *config = [WKWebViewConfiguration new];
        [config.userContentController addUserScript:[[WKUserScript alloc] initWithSource:kErrorCollector
                                                                            injectionTime:WKUserScriptInjectionTimeAtDocumentStart
                                                                         forMainFrameOnly:YES]];
        WKWebView *web = [[WKWebView alloc] initWithFrame:window.contentView.bounds configuration:config];
        web.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
        [window.contentView addSubview:web];
        Delegate *delegate = [Delegate new];
        web.navigationDelegate = delegate;
        // ES modules need http: WebKit treats file URLs as opaque origins and refuses them.
        [web loadRequest:[NSURLRequest requestWithURL:[NSURL URLWithString:path]]];
        [window orderFrontRegardless];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 90 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{ printf("timeout\n"); exit(1); });
        [app run];
    }
    return 0;
}
