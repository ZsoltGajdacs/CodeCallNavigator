package fixture;
import org.junit.Test;
import java.nio.file.Files;
import java.nio.file.Path;
public final class AppTest {
    @Test public void testOnly() throws Exception {
        Files.writeString(Path.of("tests-executed"), "unexpected test execution");
        Service.work();
    }
}
